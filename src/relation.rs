//! `parse_relation.c`: the range table and the namespace that column and
//! table references resolve against.

use pg_catalog::{Builtin, Handle, Relation, RelationKind, SearchPath};

use crate::diagnostic::{Diagnostic, Result};
use crate::types::{TypeRef, Types, Typmod};

/// `RangeTblEntry`, for the kinds this slice knows.
#[derive(Clone, Debug)]
pub struct RangeTableEntry {
    /// What the entry is.
    pub kind: RteKind,
    /// The alias the query gave, if any.
    pub alias: Option<String>,
    /// The name the entry answers to: the alias, or the relation name.
    pub eref_name: String,
    /// Column names by attribute number, starting at attnum 1; an empty
    /// name is a dropped column.
    pub colnames: Vec<String>,
    /// Byte offset of the table reference.
    pub location: usize,
    /// The entry is read as a composite type, without system columns:
    /// the `excluded` pseudo-relation of ON CONFLICT.
    pub composite_only: bool,
}

impl RangeTableEntry {
    /// The relation, when the entry is one.
    pub fn relation(&self) -> Option<Handle<Relation>> {
        match &self.kind {
            RteKind::Relation { relation, .. } => Some(*relation),
            RteKind::Join { .. }
            | RteKind::Subquery { .. }
            | RteKind::Function { .. }
            | RteKind::Values { .. }
            | RteKind::SetOperation { .. }
            | RteKind::Cte { .. } => None,
        }
    }
}

/// The kinds of range table entry, `RTEKind`.
#[derive(Clone, Debug)]
pub enum RteKind {
    /// A table, view, sequence or the like.
    Relation {
        /// The relation.
        relation: Handle<Relation>,
        /// `ONLY` was not written.
        inh: bool,
    },
    /// A reference to a WITH query.
    Cte {
        /// The WITH query's columns.
        columns: Vec<CteColumn>,
        /// The reference is the recursive one inside the query itself.
        self_reference: bool,
        /// The trailing SEARCH and CYCLE columns that `*` does not expand:
        /// `p_dontexpand`, set for a reference from inside the query.
        hidden: usize,
    },
    /// A `VALUES` list.
    Values {
        /// The type and typmod of each column.
        columns: Vec<(TypeRef, Typmod)>,
    },
    /// The result of `UNION`, `INTERSECT` or `EXCEPT`, visible to its
    /// `ORDER BY`.
    SetOperation {
        /// The type and typmod of each column.
        columns: Vec<(TypeRef, Typmod)>,
    },
    /// One or more function calls in `FROM`.
    Function {
        /// The type and typmod of each output column.
        columns: Vec<(TypeRef, Typmod)>,
        /// The composite type a single function returns, which a whole-row
        /// reference has.
        row_type: Option<TypeRef>,
    },
    /// A subquery in `FROM`.
    Subquery {
        /// The non-junk target entries of the subquery, in order.
        columns: Vec<crate::analysis::TargetEntry>,
        /// What `CheckSelectLocking` refuses for a locking clause pushed
        /// down into the subquery, if anything.
        lock_refusal: Option<&'static str>,
    },
    /// The result of a `JOIN`.
    Join {
        /// The kind of join.
        join_type: JoinType,
        /// One `Var` per output column: the input column it passes through,
        /// or a column of this entry for a merged `USING` column whose
        /// value is computed.
        columns: Vec<Var>,
        /// For each output column that is a column of this entry, the input
        /// columns its value is computed from; empty for the others. This
        /// is what `flatten_join_alias_vars` expands the column to, less
        /// the coercions.
        inputs: Vec<Vec<Var>>,
    },
}

/// `JoinType`, for the joins the grammar can write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinType {
    /// `INNER JOIN`, `JOIN`, `CROSS JOIN`.
    Inner,
    /// `LEFT [OUTER] JOIN`.
    Left,
    /// `RIGHT [OUTER] JOIN`.
    Right,
    /// `FULL [OUTER] JOIN`.
    Full,
}

/// `ParseNamespaceItem`.
#[derive(Clone, Debug)]
pub(crate) struct NamespaceItem {
    /// 1-based index into the range table.
    pub rt_index: usize,
    pub rel_visible: bool,
    pub cols_visible: bool,
    /// A `JOIN ... USING (...) AS name` alias: the name, and the number of
    /// leading columns of the join it exposes.
    pub using_alias: Option<(String, usize)>,
    /// `p_lateral_only`: visible only to a `LATERAL` item.
    pub lateral_only: bool,
    /// `p_lateral_ok`: a `LATERAL` reference to it is allowed.
    pub lateral_ok: bool,
}

impl NamespaceItem {
    /// An item that exposes the whole entry under its own name.
    pub fn plain(rt_index: usize) -> Self {
        Self {
            rt_index,
            rel_visible: true,
            cols_visible: true,
            using_alias: None,
            lateral_only: false,
            lateral_ok: true,
        }
    }
}

/// PostgreSQL's `Var`: which range table entry, which attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Var {
    /// 1-based range table index.
    pub varno: usize,
    /// Attribute number; 0 for a whole-row reference, negative for a system
    /// column.
    pub varattno: i16,
    /// How many query levels up the range table is.
    pub levelsup: usize,
    /// The attribute's type.
    pub ty: TypeRef,
    /// The attribute's typmod.
    pub typmod: Typmod,
}

/// One query level's parse state.
#[derive(Clone, Debug)]
pub(crate) struct Scope {
    pub rtable: Vec<RangeTableEntry>,
    pub namespace: Vec<NamespaceItem>,
    /// `p_lateral_active`: a `LATERAL` item of this level's FROM clause is
    /// being transformed, so the items before it are visible.
    pub lateral_active: bool,
    /// `p_ctenamespace`: the WITH queries of this level.
    pub ctes: Vec<CteEntry>,
}

/// One column of a WITH query.
#[derive(Clone, Debug)]
pub struct CteColumn {
    /// The column name.
    pub name: String,
    /// The column type.
    pub ty: TypeRef,
    /// The column typmod.
    pub typmod: Typmod,
    /// The table column it comes from, when it does.
    pub origin: Option<(Handle<Relation>, i16)>,
    /// The fields of a `record` column whose row is known.
    pub(crate) fields: Option<Vec<crate::analysis::RecordField>>,
}

/// `CommonTableExpr` as the namespace sees it.
#[derive(Clone, Debug)]
pub struct CteEntry {
    /// The WITH query name.
    pub name: String,
    /// The columns, once known.
    pub columns: Vec<CteColumn>,
    /// The columns are known: after analysis, or for a recursive query
    /// after its non-recursive term.
    pub columns_known: bool,
    /// The query is being analysed: a reference to it is a recursive one.
    pub analysing: bool,
    /// The query is INSERT, UPDATE, DELETE or MERGE.
    pub modifying: bool,
    /// A data-modifying query has a RETURNING list.
    pub has_returning: bool,
    /// Byte offset of the WITH query name.
    pub location: usize,
    /// The trailing SEARCH and CYCLE columns.
    pub hidden: usize,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            rtable: Vec::new(),
            namespace: Vec::new(),
            lateral_active: false,
            ctes: Vec::new(),
        }
    }
}

/// A system column, `SystemAttributeByName`.
pub(crate) fn system_attribute(types: &Types<'_>, name: &str) -> Option<(i16, TypeRef)> {
    let (attnum, b) = match name {
        "ctid" => (-1, Builtin::Tid),
        "xmin" => (-2, Builtin::Xid),
        "cmin" => (-3, Builtin::Cid),
        "xmax" => (-4, Builtin::Xid),
        "cmax" => (-5, Builtin::Cid),
        "tableoid" => (-6, Builtin::Oid),
        _ => return None,
    };
    Some((attnum, types.builtin(b)))
}

/// `pg_temp` names the session's temporary schema; every other name is
/// itself. `None` when `pg_temp` is named but the session has none.
pub(crate) fn resolve_schema_name<'a>(
    catalog: &'a pg_catalog::Catalog,
    name: &'a str,
) -> Option<&'a str> {
    if name == "pg_temp" {
        let temp = catalog.temporary_schema()?;
        Some(catalog[temp].name.as_str())
    } else {
        Some(name)
    }
}

/// Relation lookups over one Catalog and Search Path.
pub(crate) struct Relations<'c> {
    pub types: Types<'c>,
    pub search_path: &'c SearchPath,
}

impl<'c> Relations<'c> {
    fn catalog(&self) -> &'c pg_catalog::Catalog {
        self.types.catalog
    }

    /// `RangeVarGetRelid` with `missing_ok`: the relation a name means.
    pub fn lookup(&self, schema: Option<&str>, name: &str) -> Option<Handle<Relation>> {
        let catalog = self.catalog();
        match schema {
            Some(schema) => {
                let schema = resolve_schema_name(catalog, schema)?;
                catalog.relation_named(&pg_catalog::QualifiedName::new(schema, name))
            }
            None => catalog.relation_visible(self.search_path, name),
        }
    }

    /// `parserOpenTable` plus `addRangeTableEntry`: open the relation and
    /// make its range table entry.
    pub fn add_range_table_entry(
        &self,
        scope: &mut Scope,
        schema: Option<&str>,
        name: &str,
        alias: Option<&str>,
        column_aliases: &[String],
        inh: bool,
        location: usize,
    ) -> Result<usize> {
        let catalog = self.catalog();
        let Some(relation) = self.lookup(schema, name) else {
            let message = match schema {
                Some(schema) => format!("relation \"{schema}.{name}\" does not exist"),
                None => format!("relation \"{name}\" does not exist"),
            };
            return Err(Diagnostic::new("42P01", message).at(location).into());
        };
        let rel = &catalog[relation];
        let eref_name = alias.map_or_else(|| rel.name.to_string(), str::to_owned);
        // buildRelationAliases: dropped columns keep their attnum and get
        // an empty name; user column aliases apply to live columns in order.
        let max_attnum = rel.columns.iter().map(|c| c.position).max().unwrap_or(0);
        let mut colnames = vec![String::new(); usize::try_from(max_attnum).unwrap_or(0)];
        let mut aliases = column_aliases.iter();
        let mut numaliases_used = 0;
        for column in &rel.columns {
            let index = usize::try_from(column.position - 1).expect("attnum >= 1");
            colnames[index] = match aliases.next() {
                Some(a) => {
                    numaliases_used += 1;
                    a.clone()
                }
                None => column.name.to_string(),
            };
        }
        if aliases.next().is_some() {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "table \"{eref_name}\" has {} columns available but {} columns specified",
                    rel.columns.len(),
                    column_aliases.len()
                ),
            )
            .into());
        }
        let _ = numaliases_used;
        scope.rtable.push(RangeTableEntry {
            kind: RteKind::Relation { relation, inh },
            alias: alias.map(str::to_owned),
            eref_name,
            colnames,
            location,
            composite_only: false,
        });
        Ok(scope.rtable.len())
    }

    /// The name a namespace item answers to.
    pub fn item_name<'s>(&self, scope: &'s Scope, item: &'s NamespaceItem) -> &'s str {
        match &item.using_alias {
            Some((name, _)) => name,
            None => &scope.rtable[item.rt_index - 1].eref_name,
        }
    }

    /// `checkNameSpaceConflicts`: no two visible items of the two lists
    /// may answer to the same name, except two unaliased references to
    /// different relations.
    pub fn check_namespace_conflicts(
        &self,
        scope: &Scope,
        existing: &[NamespaceItem],
        new: &[NamespaceItem],
    ) -> Result<()> {
        for a in existing {
            if !a.rel_visible {
                continue;
            }
            let name = self.item_name(scope, a);
            let rte_a = &scope.rtable[a.rt_index - 1];
            for b in new {
                if !b.rel_visible || self.item_name(scope, b) != name {
                    continue;
                }
                let rte_b = &scope.rtable[b.rt_index - 1];
                if let (Some(ra), Some(rb)) = (rte_a.relation(), rte_b.relation())
                    && rte_a.alias.is_none()
                    && rte_b.alias.is_none()
                    && ra != rb
                {
                    continue;
                }
                return Err(Diagnostic::new(
                    "42712",
                    format!("table name \"{name}\" specified more than once"),
                )
                .into());
            }
        }
        Ok(())
    }

    /// `scanRTEForColumn`: the attnum of a column name in an entry, with
    /// system columns for relations that have them.
    pub fn scan_rte_for_column(
        &self,
        rte: &RangeTableEntry,
        colname: &str,
        location: usize,
        ncols: Option<usize>,
    ) -> Result<Option<i16>> {
        let mut result: Option<i16> = None;
        let colnames = match ncols {
            Some(n) => &rte.colnames[..n],
            None => &rte.colnames[..],
        };
        for (i, name) in colnames.iter().enumerate() {
            if name == colname {
                if result.is_some() {
                    return Err(Diagnostic::new(
                        "42702",
                        format!("column reference \"{colname}\" is ambiguous"),
                    )
                    .at(location)
                    .into());
                }
                result = Some(i as i16 + 1);
            }
        }
        if result.is_some() || ncols.is_some() {
            return Ok(result);
        }
        if let Some(relation) = rte.relation()
            && !rte.composite_only
            && self.catalog()[relation].kind != RelationKind::View
            && let Some((attnum, _)) = system_attribute(&self.types, colname)
        {
            return Ok(Some(attnum));
        }
        Ok(None)
    }

    /// The `Var` for an attnum of an entry.
    pub fn make_var(
        &self,
        rt_index: usize,
        rte: &RangeTableEntry,
        attnum: i16,
        levelsup: usize,
    ) -> Var {
        let catalog = self.catalog();
        let RteKind::Relation { relation, .. } = rte.kind else {
            let index = usize::try_from(attnum - 1).expect("attnum >= 1");
            return match &rte.kind {
                RteKind::Join { columns, .. } => Var {
                    levelsup,
                    ..columns[index]
                },
                RteKind::Subquery { columns, .. } => Var {
                    varno: rt_index,
                    varattno: attnum,
                    levelsup,
                    ty: columns[index].ty,
                    typmod: columns[index].typmod,
                },
                RteKind::Cte { columns, .. } => Var {
                    varno: rt_index,
                    varattno: attnum,
                    levelsup,
                    ty: columns[index].ty,
                    typmod: columns[index].typmod,
                },
                RteKind::Function { columns, .. }
                | RteKind::Values { columns }
                | RteKind::SetOperation { columns } => Var {
                    varno: rt_index,
                    varattno: attnum,
                    levelsup,
                    ty: columns[index].0,
                    typmod: columns[index].1,
                },
                RteKind::Relation { .. } => unreachable!("relation handled above"),
            };
        };
        let rel = &catalog[relation];
        if attnum > 0 {
            let column = rel
                .columns
                .iter()
                .find(|c| c.position == attnum)
                .expect("attnum came from colnames");
            Var {
                varno: rt_index,
                varattno: attnum,
                levelsup,
                ty: column.ty,
                typmod: column.typmod.0,
            }
        } else if attnum == 0 {
            Var {
                varno: rt_index,
                varattno: 0,
                levelsup,
                ty: rel.row_type.expect("relation with a row type"),
                typmod: -1,
            }
        } else {
            let name = match attnum {
                -1 => "ctid",
                -2 => "xmin",
                -3 => "cmin",
                -4 => "xmax",
                -5 => "cmax",
                _ => "tableoid",
            };
            let (_, ty) = system_attribute(&self.types, name).expect("system column");
            Var {
                varno: rt_index,
                varattno: attnum,
                levelsup,
                ty,
                typmod: -1,
            }
        }
    }

    /// `scanNSItemForColumn`.
    pub fn scan_ns_item_for_column(
        &self,
        scope: &Scope,
        item: &NamespaceItem,
        levelsup: usize,
        colname: &str,
        location: usize,
    ) -> Result<Option<Var>> {
        let rte = &scope.rtable[item.rt_index - 1];
        let ncols = item.using_alias.as_ref().map(|(_, n)| *n);
        let Some(attnum) = self.scan_rte_for_column(rte, colname, location, ncols)? else {
            return Ok(None);
        };
        Ok(Some(self.make_var(item.rt_index, rte, attnum, levelsup)))
    }

    /// `check_lateral_ref_ok`.
    fn lateral_ref_ok(&self, scope: &Scope, item: &NamespaceItem, location: usize) -> Result<()> {
        if item.lateral_only && !item.lateral_ok {
            return Err(Diagnostic::new(
                "42P10",
                format!(
                    "invalid reference to FROM-clause entry for table \"{}\"",
                    self.item_name(scope, item)
                ),
            )
            .detail("The combining JOIN type must be INNER or LEFT for a LATERAL reference.")
            .at(location)
            .into());
        }
        Ok(())
    }

    /// `colNameToVar`: the current level first, then each outer level.
    pub fn col_name_to_var(
        &self,
        scope: &Scope,
        outer: &[Scope],
        colname: &str,
        location: usize,
    ) -> Result<Option<Var>> {
        for (levelsup, level) in std::iter::once(scope).chain(outer.iter().rev()).enumerate() {
            let mut result: Option<Var> = None;
            for item in &level.namespace {
                if !item.cols_visible || (item.lateral_only && !level.lateral_active) {
                    continue;
                }
                if let Some(var) =
                    self.scan_ns_item_for_column(level, item, levelsup, colname, location)?
                {
                    if result.is_some() {
                        return Err(Diagnostic::new(
                            "42702",
                            format!("column reference \"{colname}\" is ambiguous"),
                        )
                        .at(location)
                        .into());
                    }
                    self.lateral_ref_ok(level, item, location)?;
                    result = Some(var);
                }
            }
            if result.is_some() {
                return Ok(result);
            }
        }
        Ok(None)
    }

    /// `refnameNamespaceItem`: the item a table name means, with how many
    /// levels up it lives.
    pub fn refname_namespace_item(
        &self,
        scope: &Scope,
        outer: &[Scope],
        schema: Option<&str>,
        refname: &str,
        location: usize,
    ) -> Result<Option<(usize, NamespaceItem)>> {
        let relid = match schema {
            Some(schema) => {
                let Some(r) = self.lookup(Some(schema), refname) else {
                    return Ok(None);
                };
                Some(r)
            }
            None => None,
        };
        for (levelsup, level) in std::iter::once(scope).chain(outer.iter().rev()).enumerate() {
            let mut result: Option<&NamespaceItem> = None;
            for item in &level.namespace {
                if !item.rel_visible || (item.lateral_only && !level.lateral_active) {
                    continue;
                }
                let rte = &level.rtable[item.rt_index - 1];
                let matches = match relid {
                    Some(r) => {
                        item.using_alias.is_none()
                            && rte.alias.is_none()
                            && rte.relation() == Some(r)
                    }
                    None => self.item_name(level, item) == refname,
                };
                if matches {
                    if result.is_some() {
                        return Err(Diagnostic::new(
                            "42P09",
                            format!("table reference \"{refname}\" is ambiguous"),
                        )
                        .at(location)
                        .into());
                    }
                    self.lateral_ref_ok(level, item, location)?;
                    result = Some(item);
                }
            }
            if let Some(item) = result {
                return Ok(Some((levelsup, item.clone())));
            }
        }
        Ok(None)
    }

    /// `errorMissingRTE`.
    pub fn error_missing_rte(
        &self,
        scope: &Scope,
        outer: &[Scope],
        schema: Option<&str>,
        relname: &str,
        location: usize,
    ) -> Diagnostic {
        // searchRangeTableForRel: an entry for the relation that is not
        // visible under this name.
        let relid = self.lookup(schema, relname);
        let found = std::iter::once(scope)
            .chain(outer.iter().rev())
            .flat_map(|level| level.rtable.iter())
            .find(|rte| {
                relid.is_some_and(|r| rte.relation() == Some(r)) || rte.eref_name == relname
            });
        match found {
            Some(rte) if rte.alias.is_some() && rte.eref_name != relname => Diagnostic::new(
                "42P01",
                format!("invalid reference to FROM-clause entry for table \"{relname}\""),
            )
            .hint(format!(
                "Perhaps you meant to reference the table alias \"{}\".",
                rte.eref_name
            ))
            .at(location),
            Some(rte) => Diagnostic::new(
                "42P01",
                format!("invalid reference to FROM-clause entry for table \"{relname}\""),
            )
            .detail(format!(
                "There is an entry for table \"{}\", but it cannot be referenced from this part of the query.",
                rte.eref_name
            ))
            .at(location),
            None => Diagnostic::new(
                "42P01",
                format!("missing FROM-clause entry for table \"{relname}\""),
            )
            .at(location),
        }
    }

    /// `errorMissingColumn`, without the fuzzy-match hint.
    pub fn error_missing_column(
        &self,
        relname: Option<&str>,
        colname: &str,
        location: usize,
    ) -> Diagnostic {
        let message = match relname {
            Some(relname) => format!("column {relname}.{colname} does not exist"),
            None => format!("column \"{colname}\" does not exist"),
        };
        Diagnostic::new("42703", message).at(location)
    }
}
