//! `parse_relation.c`: the range table and the namespace that column and
//! table references resolve against.

use pg_catalog::{Builtin, Handle, Relation, RelationKind, SearchPath};

use crate::diagnostic::{Diagnostic, Result};
use crate::types::{TypeRef, Types, Typmod};

/// `RangeTblEntry`, for the kinds this slice knows.
#[derive(Clone, Debug)]
pub struct RangeTableEntry {
    /// The relation, for a table, view, sequence or the like.
    pub relation: Handle<Relation>,
    /// The alias the query gave, if any.
    pub alias: Option<String>,
    /// The name the entry answers to: the alias, or the relation name.
    pub eref_name: String,
    /// Column names by attribute number, starting at attnum 1; an empty
    /// name is a dropped column.
    pub colnames: Vec<String>,
    /// `ONLY` was not written.
    pub inh: bool,
    /// Byte offset of the table reference.
    pub location: usize,
}

/// `ParseNamespaceItem`.
#[derive(Clone, Debug)]
pub(crate) struct NamespaceItem {
    /// 1-based index into the range table.
    pub rt_index: usize,
    pub rel_visible: bool,
    pub cols_visible: bool,
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
pub(crate) struct Scope {
    pub rtable: Vec<RangeTableEntry>,
    pub namespace: Vec<NamespaceItem>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            rtable: Vec::new(),
            namespace: Vec::new(),
        }
    }
}

/// A system column, `SystemAttributeByName`.
fn system_attribute(types: &Types<'_>, name: &str) -> Option<(i16, TypeRef)> {
    let (attnum, b) = match name {
        "ctid" => (-1, Builtin::Tid),
        "xmin" => (-3, Builtin::Xid),
        "cmin" => (-4, Builtin::Cid),
        "xmax" => (-5, Builtin::Xid),
        "cmax" => (-6, Builtin::Cid),
        "tableoid" => (-7, Builtin::Oid),
        _ => return None,
    };
    Some((attnum, types.builtin(b)))
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
            Some(schema) => catalog.relation_named(&pg_catalog::QualifiedName::new(schema, name)),
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
            relation,
            alias: alias.map(str::to_owned),
            eref_name,
            colnames,
            inh,
            location,
        });
        Ok(scope.rtable.len())
    }

    /// `checkNameSpaceConflicts` for one new item against the namespace.
    pub fn check_namespace_conflicts(&self, scope: &Scope, new_index: usize) -> Result<()> {
        let new = &scope.rtable[new_index - 1];
        for item in &scope.namespace {
            if !item.rel_visible {
                continue;
            }
            let existing = &scope.rtable[item.rt_index - 1];
            if existing.eref_name == new.eref_name {
                // Two unaliased references to the same relation are allowed
                // only when they are the same entry, which they never are here.
                return Err(Diagnostic::new(
                    "42712",
                    format!("table name \"{}\" specified more than once", new.eref_name),
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
    ) -> Result<Option<i16>> {
        let mut result: Option<i16> = None;
        for (i, name) in rte.colnames.iter().enumerate() {
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
        if result.is_some() {
            return Ok(result);
        }
        let kind = self.catalog()[rte.relation].kind;
        if kind != RelationKind::View
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
        let rel = &catalog[rte.relation];
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
                -3 => "xmin",
                -4 => "cmin",
                -5 => "xmax",
                -6 => "cmax",
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
        let Some(attnum) = self.scan_rte_for_column(rte, colname, location)? else {
            return Ok(None);
        };
        Ok(Some(self.make_var(item.rt_index, rte, attnum, levelsup)))
    }

    /// `colNameToVar` over one level (subqueries are out of scope).
    pub fn col_name_to_var(
        &self,
        scope: &Scope,
        colname: &str,
        location: usize,
    ) -> Result<Option<Var>> {
        let mut result: Option<Var> = None;
        for item in &scope.namespace {
            if !item.cols_visible {
                continue;
            }
            if let Some(var) = self.scan_ns_item_for_column(scope, item, 0, colname, location)? {
                if result.is_some() {
                    return Err(Diagnostic::new(
                        "42702",
                        format!("column reference \"{colname}\" is ambiguous"),
                    )
                    .at(location)
                    .into());
                }
                result = Some(var);
            }
        }
        Ok(result)
    }

    /// `refnameNamespaceItem` over one level.
    pub fn refname_namespace_item<'s>(
        &self,
        scope: &'s Scope,
        schema: Option<&str>,
        refname: &str,
        location: usize,
    ) -> Result<Option<&'s NamespaceItem>> {
        let catalog = self.catalog();
        let relid = match schema {
            Some(schema) => {
                let Some(r) =
                    catalog.relation_named(&pg_catalog::QualifiedName::new(schema, refname))
                else {
                    return Ok(None);
                };
                Some(r)
            }
            None => None,
        };
        let mut result: Option<&NamespaceItem> = None;
        for item in &scope.namespace {
            if !item.rel_visible {
                continue;
            }
            let rte = &scope.rtable[item.rt_index - 1];
            let matches = match relid {
                Some(r) => rte.relation == r,
                None => rte.eref_name == refname,
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
                result = Some(item);
            }
        }
        Ok(result)
    }

    /// `errorMissingRTE`.
    pub fn error_missing_rte(
        &self,
        scope: &Scope,
        schema: Option<&str>,
        relname: &str,
        location: usize,
    ) -> Diagnostic {
        // searchRangeTableForRel: an entry for the relation that is not
        // visible under this name.
        let relid = self.lookup(schema, relname);
        let found = scope
            .rtable
            .iter()
            .find(|rte| relid.is_some_and(|r| rte.relation == r) || rte.eref_name == relname);
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
