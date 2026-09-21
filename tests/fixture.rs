//! Tests of the embedded stock PostgreSQL 17 catalog.
//!
//! ```sh
//! cargo test --features fixture --test fixture
//! ```
//!
//! The file needs the `fixture` feature. Without it it holds no test.

#![cfg(feature = "fixture")]

use pg_catalog::{
    Builtin, CastContext, Catalog, CatalogBuilder, Column, CompositeDetail, Field, FunctionKind,
    Name, Oid, QualifiedName, Relation, RelationKind, Schema, SearchPath, Type, TypeCategory,
    TypeKind, Typmod, fixture::pg17,
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
        array: None,
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
            identity: false,
        }],
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
