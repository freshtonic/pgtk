//! Tests of the embedded stock PostgreSQL 17 catalog.
//!
//! ```sh
//! cargo test --features fixture --test fixture
//! ```
//!
//! The file needs the `fixture` feature. Without it it holds no test.

#![cfg(feature = "fixture")]

use pgtk_catalog::{
    Builtin, CastContext, Catalog, CatalogBuilder, CollationProvider, Column, CompositeDetail,
    ConstraintKind, Field, FiresWhen, FunctionKind, Name, Oid, ParameterKind, QualifiedName,
    Relation, RelationKind, RuleEvent, Schema, SearchPath, Type, TypeCategory, TypeKind, Typmod,
    fixture::pg17,
};

#[test]
fn it_reads_the_current_format_version() {
    let catalog = pg17();
    assert_eq!(
        catalog.metadata().format_version,
        Catalog::FORMAT_VERSION,
        "regenerate the fixture with `cargo xtask fixture`"
    );
}

#[test]
fn it_holds_a_postgresql_17_server() {
    let catalog = pg17();
    assert_eq!(catalog.metadata().server_version.major, 17);
}

#[test]
fn int4_is_a_builtin_in_pg_catalog() {
    let catalog = pg17();
    let handle = catalog.builtin(Builtin::Int4);
    let name = catalog.qualified_name_of_type(handle);
    assert_eq!(name.schema.as_str(), "pg_catalog");
    assert_eq!(name.name.as_str(), "int4");
    assert_eq!(catalog.as_builtin(handle), Some(Builtin::Int4));
}

#[test]
fn the_array_of_int4_is_int4_array() {
    let catalog = pg17();
    let int4 = catalog.builtin(Builtin::Int4);
    let array = catalog[int4].array.expect("int4 has an array type");
    assert_eq!(catalog.as_builtin(array), Some(Builtin::Int4Array));
    assert_eq!(catalog[array].kind, TypeKind::Array { element: int4 });
}

#[test]
fn int4_has_the_btree_operators_and_a_hash_equality() {
    let catalog = pg17();
    let int4 = catalog.builtin(Builtin::Int4);
    let boolean = catalog.builtin(Builtin::Bool);

    let btree = catalog
        .btree_ordering(int4)
        .expect("int4 has a default btree operator class");
    let members = [
        (btree.lt, "<"),
        (btree.le, "<="),
        (btree.eq, "="),
        (btree.ge, ">="),
        (btree.gt, ">"),
    ];
    for (handle, symbol) in members {
        let operator = &catalog[handle];
        assert_eq!(operator.name.as_str(), symbol);
        assert_eq!(operator.left, Some(int4), "{symbol} takes int4 on the left");
        assert_eq!(operator.right, int4, "{symbol} takes int4 on the right");
        assert_eq!(operator.result, boolean);
    }

    // The hash class and the btree class agree on the equality operator.
    assert_eq!(catalog.hash_equality(int4), Some(btree.eq));
    assert_eq!(catalog[int4].btree, Some(btree));
}

#[test]
fn json_and_xml_have_no_ordering_and_no_equality() {
    let catalog = pg17();
    for builtin in [Builtin::Json, Builtin::Xml] {
        let ty = catalog.builtin(builtin);
        assert_eq!(catalog.btree_ordering(ty), None, "{builtin:?}");
        assert_eq!(catalog.hash_equality(ty), None, "{builtin:?}");
    }
}

#[test]
fn xid_has_a_hash_equality_and_no_ordering() {
    let catalog = pg17();
    let xid = catalog.builtin(Builtin::Xid);
    assert_eq!(catalog.btree_ordering(xid), None);
    let eq = catalog.hash_equality(xid).expect("xid has a hash class");
    assert_eq!(catalog[eq].name.as_str(), "=");
    assert_eq!(catalog[eq].right, xid);
}

#[test]
fn the_operator_class_lookup_is_for_the_exact_type() {
    let catalog = pg17();

    // `varchar` sorts through the class of `text`, because it is
    // binary-coercible to `text`. The lookup does not follow that rule.
    assert_eq!(
        catalog.btree_ordering(catalog.builtin(Builtin::Varchar)),
        None
    );
    assert!(
        catalog
            .btree_ordering(catalog.builtin(Builtin::Text))
            .is_some()
    );

    // An array sorts through the class of `anyarray`.
    assert_eq!(
        catalog.btree_ordering(catalog.builtin(Builtin::Int4Array)),
        None
    );
    let anyarray = catalog.builtin(Builtin::Anyarray);
    let btree = catalog
        .btree_ordering(anyarray)
        .expect("anyarray has a default btree operator class");
    assert_eq!(catalog[btree.lt].name.as_str(), "<");
    assert_eq!(catalog[btree.lt].right, anyarray);
    assert!(catalog.hash_equality(anyarray).is_some());
}

#[test]
fn pg_class_has_its_oid_as_the_primary_key() {
    let catalog = pg17();
    let handle = catalog
        .relation_named(&QualifiedName::new("pg_catalog", "pg_class"))
        .expect("pg_class is a relation");
    let relation = &catalog[handle];
    let key = relation
        .primary_key
        .as_ref()
        .expect("pg_class has a primary key");
    assert!(!key.deferrable);
    let names: Vec<&str> = key
        .grouping_columns()
        .expect("the key is not deferrable")
        .iter()
        .map(|position| relation.column_at(*position).unwrap().name.as_str())
        .collect();
    assert_eq!(names, vec!["oid"]);

    // A view has no primary key.
    let view = catalog
        .relation_named(&QualifiedName::new("pg_catalog", "pg_tables"))
        .expect("pg_tables is a relation");
    assert_eq!(catalog[view].primary_key, None);
}

#[test]
fn make_interval_has_the_type_of_each_default() {
    let catalog = pg17();
    let int4 = catalog.builtin(Builtin::Int4);
    let float8 = catalog.builtin(Builtin::Float8);
    let overloads = catalog.functions_named(&QualifiedName::new("pg_catalog", "make_interval"));
    assert_eq!(overloads.len(), 1);
    let function = &catalog[overloads[0]];

    // Each of the seven arguments has a default: six `int4` and one `float8`.
    assert_eq!(function.default_count, 7);
    let mut expected = vec![Some(int4); 6];
    expected.push(Some(float8));
    assert_eq!(function.default_types, expected);
    assert_eq!(function.default_type_of(0), Some(int4));
    assert_eq!(function.default_type_of(6), Some(float8));
}

#[test]
fn each_function_has_one_default_type_for_each_default() {
    let catalog = pg17();
    let mut with_defaults = 0;
    for (_, function) in catalog.functions() {
        assert_eq!(
            function.default_types.len(),
            usize::from(function.default_count),
            "{}",
            function.name
        );
        if function.default_count > 0 {
            with_defaults += 1;
            // A stock server has no default that the loader cannot read.
            assert!(
                function.default_types.iter().all(Option::is_some),
                "{}",
                function.name
            );
        }
    }
    assert!(
        with_defaults > 20,
        "{with_defaults} functions have a default"
    );
}

#[test]
fn pg_settings_is_a_view_with_two_update_rules() {
    let catalog = pg17();
    let handle = catalog
        .relation_named(&QualifiedName::new("pg_catalog", "pg_settings"))
        .expect("pg_settings is a relation");
    let view = &catalog[handle];
    assert_eq!(view.kind, RelationKind::View);
    let text = view
        .view_definition
        .as_deref()
        .expect("a view has its text");
    assert!(text.contains("pg_show_all_settings"), "{text}");

    // The `_RETURN` rule is not here. The two other rules are in name order.
    let rules: Vec<(&str, RuleEvent, bool, bool)> = view
        .rules
        .iter()
        .map(|rule| {
            (
                rule.name.as_str(),
                rule.event,
                rule.instead,
                rule.unconditional,
            )
        })
        .collect();
    assert_eq!(
        rules,
        vec![
            ("pg_settings_n", RuleEvent::Update, true, true),
            ("pg_settings_u", RuleEvent::Update, false, false),
        ]
    );
    assert!(view.rules[0].replaces(RuleEvent::Update));
    assert!(
        view.rules
            .iter()
            .all(|rule| rule.fires == FiresWhen::Origin)
    );
    assert_eq!(view.triggers, Vec::new());
}

#[test]
fn only_a_view_has_view_text() {
    let catalog = pg17();
    for (_, relation) in catalog.relations() {
        let is_view = matches!(
            relation.kind,
            RelationKind::View | RelationKind::MaterializedView
        );
        assert_eq!(
            relation.view_definition.is_some(),
            is_view,
            "{}",
            relation.name
        );
    }
}

#[test]
fn show_timezone_has_the_canonical_name() {
    let catalog = pg17();
    let parameter = catalog.parameter("timezone").expect("TimeZone is there");
    assert_eq!(parameter.name, "TimeZone");
    assert_eq!(parameter.kind, ParameterKind::String);

    assert_eq!(
        catalog.parameter("work_mem").map(|p| p.kind),
        Some(ParameterKind::Integer)
    );
    assert_eq!(
        catalog.parameter("enable_seqscan").map(|p| p.kind),
        Some(ParameterKind::Bool)
    );
    assert_eq!(
        catalog.parameter("client_min_messages").map(|p| p.kind),
        Some(ParameterKind::Enum)
    );
    assert_eq!(
        catalog.parameter("seq_page_cost").map(|p| p.kind),
        Some(ParameterKind::Real)
    );
    assert_eq!(catalog.parameter("no_such_parameter"), None);
    assert!(catalog.parameters().len() > 300);
}

#[test]
fn the_parameters_survive_a_serde_round_trip() {
    let catalog = pg17();
    let json = serde_json::to_string(&catalog).expect("the catalog serializes");
    let again: Catalog = serde_json::from_str(&json).expect("the catalog deserializes");
    assert_eq!(again.parameters(), catalog.parameters());
}

#[test]
fn text_has_the_default_collation_and_int4_has_none() {
    let catalog = pg17();
    assert_eq!(catalog.metadata().encoding.name, "UTF8");

    let default = catalog
        .collation_named(&QualifiedName::new("pg_catalog", "default"))
        .expect("the default collation is there");
    assert_eq!(catalog[default].oid, Oid(100));
    assert_eq!(catalog[default].provider, CollationProvider::Default);
    assert_eq!(catalog[default].encoding, None);

    let text = catalog.builtin(Builtin::Text);
    assert!(catalog[text].is_collatable);
    assert_eq!(catalog[text].collation, Some(default));
    let int4 = catalog.builtin(Builtin::Int4);
    assert!(!catalog[int4].is_collatable);
    assert_eq!(catalog[int4].collation, None);

    // A type is collatable exactly when it has a collation.
    for (_, ty) in catalog.types() {
        assert_eq!(ty.is_collatable, ty.collation.is_some(), "{}", ty.name);
    }
}

#[test]
fn c_and_posix_are_visible_collations() {
    let catalog = pg17();
    let path = SearchPath::from_metadata(&catalog);
    let c = catalog.collation_visible(&path, "C").expect("C is there");
    let posix = catalog
        .collation_visible(&path, "POSIX")
        .expect("POSIX is there");
    assert_ne!(c, posix);
    assert_eq!(catalog[c].oid, Oid(950));
    assert_eq!(catalog[posix].oid, Oid(951));
    assert!(catalog[c].deterministic);
    assert_eq!(catalog.collation_visible(&path, "no_such_collation"), None);

    // `pg_class.relname` has the type `name`, whose collation is `C`.
    let pg_class = catalog
        .relation_named(&QualifiedName::new("pg_catalog", "pg_class"))
        .expect("pg_class is a relation");
    let relname = catalog[pg_class]
        .column("relname")
        .expect("relname is there");
    assert_eq!(relname.collation, Some(c));
    let oid = catalog[pg_class].column("oid").expect("oid is there");
    assert_eq!(oid.collation, None);
}

#[test]
fn pg_class_has_its_constraints_by_name() {
    let catalog = pg17();
    let handle = catalog
        .relation_named(&QualifiedName::new("pg_catalog", "pg_class"))
        .expect("pg_class is a relation");
    let relation = &catalog[handle];

    let key = relation
        .constraint("pg_class_oid_index")
        .expect("the primary key is a constraint");
    assert_eq!(key.kind, ConstraintKind::PrimaryKey);
    assert_eq!(
        Some(key.columns.as_slice()),
        relation.primary_key.as_ref().map(|k| k.columns.as_slice())
    );

    let unique = relation
        .constraint("pg_class_relname_nsp_index")
        .expect("the unique constraint is there");
    assert_eq!(unique.kind, ConstraintKind::Unique);
    assert_eq!(unique.columns.len(), 2);
    assert!(relation.constraint("no_such_constraint").is_none());

    // Each primary key is also a constraint.
    for (_, relation) in catalog.relations() {
        let keys = relation
            .constraints
            .iter()
            .filter(|c| c.kind == ConstraintKind::PrimaryKey)
            .count();
        assert_eq!(
            keys,
            usize::from(relation.primary_key.is_some()),
            "{}",
            relation.name
        );
    }
}

#[test]
fn int4_ops_is_a_btree_class_and_a_hash_class() {
    let catalog = pg17();
    let int4 = catalog.builtin(Builtin::Int4);
    let path = SearchPath::from_metadata(&catalog);
    for method in ["btree", "hash"] {
        let handle = catalog
            .operator_class_visible(&path, "int4_ops", method)
            .unwrap_or_else(|| panic!("int4_ops is a {method} class"));
        let class = &catalog[handle];
        assert_eq!(class.method.as_str(), method);
        assert_eq!(class.input_type, int4);
        assert!(class.is_default);
    }
    assert_eq!(
        catalog.operator_class_visible(&path, "int4_ops", "gist"),
        None
    );
    assert_eq!(
        catalog.operator_class_visible(&path, "no_such_ops", "btree"),
        None
    );

    // `text_pattern_ops` is a class that is not the default.
    let pattern = catalog
        .operator_class_named(
            &QualifiedName::new("pg_catalog", "text_pattern_ops"),
            "btree",
        )
        .expect("text_pattern_ops is there");
    assert!(!catalog[pattern].is_default);
    assert!(catalog.operator_classes().len() > 150);
}

#[test]
fn a_rule_has_its_definition() {
    let catalog = pg17();
    let handle = catalog
        .relation_named(&QualifiedName::new("pg_catalog", "pg_settings"))
        .expect("pg_settings is a relation");
    for rule in &catalog[handle].rules {
        assert!(
            rule.definition.starts_with("CREATE RULE"),
            "{}",
            rule.definition
        );
        assert!(rule.definition.contains(rule.name.as_str()));
    }
}

#[test]
fn count_is_visible_and_has_an_aggregate() {
    let catalog = pg17();
    let path = SearchPath::from_metadata(&catalog);
    let visible = catalog.functions_visible(&path, "count");
    assert!(
        visible
            .iter()
            .any(|handle| catalog[*handle].kind == FunctionKind::Aggregate),
        "count has an aggregate variant"
    );
}

#[test]
fn the_cast_from_int4_to_int8_is_implicit() {
    let catalog = pg17();
    let int4 = catalog.builtin(Builtin::Int4);
    let int8 = catalog.builtin(Builtin::Int8);
    let cast = catalog.cast(int4, int8).expect("int4 casts to int8");
    assert_eq!(catalog[cast].context, CastContext::Implicit);
}

#[test]
fn pg_class_is_a_relation_with_a_relname_column() {
    let catalog = pg17();
    let name = QualifiedName::new("pg_catalog", "pg_class");
    let handle = catalog
        .relation_named(&name)
        .expect("pg_class is a relation");
    let relation = &catalog[handle];
    assert_eq!(relation.kind, RelationKind::Table);
    let column = relation.column("relname").expect("pg_class has relname");
    assert_eq!(
        catalog.qualified_name_of_type(column.ty).name.as_str(),
        "name"
    );
}

/// Compare two catalogs arena by arena.
///
/// The arenas are private, so the comparison reads each one through its
/// iterator. Each iterator gives the entities in handle order.
fn assert_same_catalog(left: &Catalog, right: &Catalog) {
    assert_eq!(left.metadata(), right.metadata(), "metadata");

    macro_rules! same_arena {
        ($arena:ident) => {{
            assert_eq!(
                left.$arena().len(),
                right.$arena().len(),
                concat!(stringify!($arena), " count")
            );
            for ((handle, a), (other, b)) in left.$arena().zip(right.$arena()) {
                assert_eq!(handle, other, concat!(stringify!($arena), " handle"));
                assert_eq!(a, b, "{} {:?}", stringify!($arena), handle);
            }
        }};
    }

    same_arena!(schemas);
    same_arena!(extensions);
    same_arena!(types);
    same_arena!(relations);
    same_arena!(functions);
    same_arena!(operators);
    same_arena!(casts);
}

#[test]
fn the_fixture_makes_a_serde_round_trip() {
    let catalog = pg17();
    let json = serde_json::to_string(&catalog).expect("the catalog serializes");
    let back: Catalog = serde_json::from_str(&json).expect("the catalog deserializes");

    assert_same_catalog(&catalog, &back);

    // The rebuilt catalog answers a lookup the same way, so its indexes and
    // its built-ins came back too.
    let int4 = back.builtin(Builtin::Int4);
    assert_eq!(back.as_builtin(int4), Some(Builtin::Int4));
    assert_eq!(catalog.builtin(Builtin::Int4), int4);
    let name = QualifiedName::new("pg_catalog", "pg_class");
    assert_eq!(back.relation_named(&name), catalog.relation_named(&name));
}

#[test]
fn the_builder_adds_a_table_to_the_fixture() {
    let catalog = pg17();
    let int4 = catalog.builtin(Builtin::Int4);
    let mut builder = CatalogBuilder::from_catalog(catalog);
    let schema = builder.push_schema(Schema {
        oid: Oid(900_000),
        name: Name::new("app"),
        is_temporary: false,
        extension: None,
    });
    let row_type = builder.next_type_handle();
    let relation = builder.next_relation_handle();
    builder.push_type(Type {
        oid: Oid(900_001),
        name: Name::new("widget"),
        schema,
        category: TypeCategory::Composite,
        is_preferred: false,
        is_collatable: false,
        collation: None,
        array: None,
        btree: None,
        hash_equality: None,
        kind: TypeKind::Composite(CompositeDetail {
            fields: vec![Field {
                name: Name::new("id"),
                ty: int4,
                typmod: Typmod::NONE,
            }],
            relation: Some(relation),
        }),
        extension: None,
    });
    builder.push_relation(Relation {
        oid: Oid(900_002),
        name: Name::new("widget"),
        schema,
        kind: RelationKind::Table,
        row_type: Some(row_type),
        columns: vec![Column {
            name: Name::new("id"),
            position: 1,
            ty: int4,
            typmod: Typmod::NONE,
            not_null: true,
            has_default: false,
            generated: false,
            collation: None,
            identity: None,
        }],
        of_type: None,
        parents: Vec::new(),
        primary_key: None,
        constraints: Vec::new(),
        view_definition: None,
        rules: Vec::new(),
        triggers: Vec::new(),
        extension: None,
    });

    let catalog = builder.finish().expect("the catalog is valid");
    let name = QualifiedName::new("app", "widget");
    let handle = catalog.relation_named(&name).expect("app.widget is there");
    assert_eq!(catalog[handle].column("id").unwrap().ty, int4);
    assert_eq!(
        catalog.as_builtin(catalog.builtin(Builtin::Text)),
        Some(Builtin::Text)
    );
}
