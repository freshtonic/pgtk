//! Build a small catalog by hand, then exercise every lookup.

use pg_catalog::{
    ArgMode, Argument, Builtin, Cast, CastContext, CastMethod, Catalog, CatalogBuilder, Column,
    CompositeDetail, DomainDetail, Extension, Field, Function, FunctionKind, Handle, Metadata,
    Name, Oid, Operator, OperatorKind, QualifiedName, Relation, RelationKind, Schema, SearchPath,
    Type, TypeCategory, TypeKind, Typmod, Volatility,
};

/// The handles of the entities that [`small_catalog`] builds.
struct Handles {
    pg_catalog: Handle<Schema>,
    public: Handle<Schema>,
    widgets: Handle<Extension>,
    int4: Handle<Type>,
    int4_array: Handle<Type>,
    text: Handle<Type>,
    positive: Handle<Type>,
    mood: Handle<Type>,
    widget_row: Handle<Type>,
    widget: Handle<Relation>,
    catalog_f_int4: Handle<Function>,
    catalog_f_text: Handle<Function>,
    public_f_int4: Handle<Function>,
    public_f_text: Handle<Function>,
    plus: Handle<Operator>,
    int4_to_mood: Handle<Cast>,
    int4_to_text: Handle<Cast>,
}

fn base_type(oid: u32, name: &str, schema: Handle<Schema>, category: TypeCategory) -> Type {
    Type {
        oid: Oid(oid),
        name: Name::new(name),
        schema,
        category,
        is_preferred: false,
        is_collatable: false,
        array: None,
        kind: TypeKind::Base,
        extension: None,
    }
}

fn function(
    oid: u32,
    name: &str,
    schema: Handle<Schema>,
    argument: Handle<Type>,
    returns: Handle<Type>,
) -> Function {
    Function {
        oid: Oid(oid),
        name: Name::new(name),
        schema,
        kind: FunctionKind::Function,
        arguments: vec![Argument {
            name: Some(Name::new("a")),
            ty: argument,
            mode: ArgMode::In,
        }],
        variadic: None,
        default_count: 0,
        returns,
        returns_set: false,
        strict: true,
        volatility: Volatility::Immutable,
        aggregate: None,
        extension: None,
    }
}

/// A catalog with two schemas, one extension, six types, one relation, four
/// functions, one operator, and two casts.
fn small_catalog() -> (Catalog, Handles) {
    let mut builder = CatalogBuilder::new(Metadata {
        search_path: vec![Name::new("public")],
        ..Metadata::placeholder()
    });
    builder.permit_missing_builtins();

    let pg_catalog = builder.push_schema(Schema {
        oid: Oid(11),
        name: Name::new("pg_catalog"),
        extension: None,
    });
    let public = builder.push_schema(Schema {
        oid: Oid(2200),
        name: Name::new("public"),
        extension: None,
    });

    let widgets = builder.push_extension(Extension {
        oid: Oid(60000),
        name: Name::new("widgets"),
        version: "1.0".to_owned(),
        schema: public,
    });

    // `int4` and `int4[]` point at each other. The array handle comes from
    // the push that follows, so `type_mut` closes the cycle.
    let int4 = builder.push_type(base_type(23, "int4", pg_catalog, TypeCategory::Numeric));
    let int4_array = builder.push_type(Type {
        kind: TypeKind::Array { element: int4 },
        ..base_type(1007, "_int4", pg_catalog, TypeCategory::Array)
    });
    builder.type_mut(int4).array = Some(int4_array);

    let text = builder.push_type(base_type(25, "text", pg_catalog, TypeCategory::String));

    let positive = builder.push_type(Type {
        kind: TypeKind::Domain(DomainDetail {
            base: int4,
            typmod: Typmod::NONE,
            not_null: true,
            check_constraints: 1,
        }),
        ..base_type(20000, "positive_int", public, TypeCategory::Numeric)
    });

    let mood = builder.push_type(Type {
        kind: TypeKind::Enum {
            labels: vec![Name::new("sad"), Name::new("ok"), Name::new("happy")],
        },
        extension: Some(widgets),
        ..base_type(20001, "mood", public, TypeCategory::Enum)
    });

    // The composite row type and its relation point at each other. Both
    // handles are known before either push.
    let widget_row = builder.next_type_handle();
    let widget = builder.next_relation_handle();
    let pushed_row = builder.push_type(Type {
        kind: TypeKind::Composite(CompositeDetail {
            fields: vec![
                Field {
                    name: Name::new("id"),
                    ty: int4,
                    typmod: Typmod::NONE,
                },
                Field {
                    name: Name::new("label"),
                    ty: text,
                    typmod: Typmod(14),
                },
            ],
            relation: Some(widget),
        }),
        ..base_type(20002, "widget", public, TypeCategory::Composite)
    });
    assert_eq!(pushed_row, widget_row);
    let pushed_relation = builder.push_relation(Relation {
        oid: Oid(20003),
        name: Name::new("widget"),
        schema: public,
        kind: RelationKind::Table,
        row_type: widget_row,
        columns: vec![
            Column {
                name: Name::new("id"),
                position: 1,
                ty: int4,
                typmod: Typmod::NONE,
                not_null: true,
                has_default: false,
                generated: false,
                identity: true,
            },
            Column {
                name: Name::new("label"),
                position: 2,
                ty: text,
                typmod: Typmod(14),
                not_null: false,
                has_default: true,
                generated: false,
                identity: false,
            },
        ],
        extension: Some(widgets),
    });
    assert_eq!(pushed_relation, widget);

    // Push each pair of overloads out of OID order, so that the name index
    // has to sort them.
    let catalog_f_text = builder.push_function(function(30001, "f", pg_catalog, text, text));
    let catalog_f_int4 = builder.push_function(function(30000, "f", pg_catalog, int4, int4));
    let public_f_text = builder.push_function(function(30003, "f", public, text, text));
    let public_f_int4 = builder.push_function(function(30002, "f", public, int4, int4));

    let plus = builder.push_operator(Operator {
        oid: Oid(40000),
        name: Name::new("+"),
        schema: pg_catalog,
        kind: OperatorKind::Infix,
        left: Some(int4),
        right: int4,
        result: int4,
        function: catalog_f_int4,
        commutator: None,
        negator: None,
        extension: None,
    });
    builder.operator_mut(plus).commutator = Some(plus);

    let int4_to_text = builder.push_cast(Cast {
        oid: Oid(50001),
        source: int4,
        target: text,
        context: CastContext::Implicit,
        method: CastMethod::Function(catalog_f_text),
        extension: None,
    });
    let int4_to_mood = builder.push_cast(Cast {
        oid: Oid(50000),
        source: int4,
        target: mood,
        context: CastContext::Explicit,
        method: CastMethod::Binary,
        extension: Some(widgets),
    });

    let catalog = builder.finish().expect("the catalog builds");
    (
        catalog,
        Handles {
            pg_catalog,
            public,
            widgets,
            int4,
            int4_array,
            text,
            positive,
            mood,
            widget_row,
            widget,
            catalog_f_int4,
            catalog_f_text,
            public_f_int4,
            public_f_text,
            plus,
            int4_to_mood,
            int4_to_text,
        },
    )
}

#[test]
fn metadata_survives_the_build() {
    let (catalog, _) = small_catalog();
    assert_eq!(catalog.metadata().database.as_str(), "placeholder");
    assert_eq!(catalog.metadata().format_version, Catalog::FORMAT_VERSION);
    assert_eq!(catalog.metadata().search_path, vec![Name::new("public")]);
}

#[test]
fn indexing_reads_every_arena() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog[h.pg_catalog].name.as_str(), "pg_catalog");
    assert_eq!(catalog[h.widgets].version, "1.0");
    assert_eq!(catalog[h.int4].name.as_str(), "int4");
    assert_eq!(catalog[h.widget].columns.len(), 2);
    assert_eq!(catalog[h.catalog_f_int4].oid, Oid(30000));
    assert_eq!(catalog[h.plus].name.as_str(), "+");
    assert_eq!(catalog[h.int4_to_text].context, CastContext::Implicit);
}

#[test]
fn the_builtin_lookups_use_the_oid() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog.builtin(Builtin::Int4), h.int4);
    assert_eq!(catalog.try_builtin(Builtin::Int4), Some(h.int4));
    assert_eq!(catalog.try_builtin(Builtin::Int4Array), Some(h.int4_array));
    assert_eq!(catalog.try_builtin(Builtin::Bool), None);
    assert_eq!(catalog.as_builtin(h.text), Some(Builtin::Text));
    assert_eq!(catalog.as_builtin(h.mood), None);
}

#[test]
#[should_panic(expected = "built-in type Bool")]
fn asking_for_an_absent_builtin_panics() {
    let (catalog, _) = small_catalog();
    let _ = catalog.builtin(Builtin::Bool);
}

#[test]
fn every_arena_iterates() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog.schemas().len(), 2);
    assert_eq!(catalog.extensions().len(), 1);
    assert_eq!(catalog.types().len(), 6);
    assert_eq!(catalog.relations().len(), 1);
    assert_eq!(catalog.functions().len(), 4);
    assert_eq!(catalog.operators().len(), 1);
    assert_eq!(catalog.casts().len(), 2);

    let names: Vec<&str> = catalog.schemas().map(|(_, s)| s.name.as_str()).collect();
    assert_eq!(names, vec!["pg_catalog", "public"]);
    let first = catalog.types().next().unwrap();
    assert_eq!(first.0, h.int4);
}

#[test]
fn every_arena_answers_an_oid() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog.schema_by_oid(Oid(2200)), Some(h.public));
    assert_eq!(catalog.extension_by_oid(Oid(60000)), Some(h.widgets));
    assert_eq!(catalog.type_by_oid(Oid(20000)), Some(h.positive));
    assert_eq!(catalog.relation_by_oid(Oid(20003)), Some(h.widget));
    assert_eq!(catalog.function_by_oid(Oid(30003)), Some(h.public_f_text));
    assert_eq!(catalog.operator_by_oid(Oid(40000)), Some(h.plus));
    assert_eq!(catalog.cast_by_oid(Oid(50000)), Some(h.int4_to_mood));
    assert_eq!(catalog.type_by_oid(Oid(999_999)), None);
}

#[test]
fn schemas_and_extensions_answer_a_bare_name() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog.schema_named("public"), Some(h.public));
    assert_eq!(catalog.schema_named("nope"), None);
    assert_eq!(catalog.extension_named("widgets"), Some(h.widgets));
    assert_eq!(catalog.extension_named("nope"), None);
}

#[test]
fn types_and_relations_answer_a_qualified_name() {
    let (catalog, h) = small_catalog();
    assert_eq!(
        catalog.type_named(&QualifiedName::new("pg_catalog", "int4")),
        Some(h.int4)
    );
    assert_eq!(
        catalog.type_named(&QualifiedName::new("public", "mood")),
        Some(h.mood)
    );
    assert_eq!(
        catalog.type_named(&QualifiedName::new("public", "int4")),
        None
    );
    assert_eq!(
        catalog.type_named(&QualifiedName::new("nope", "int4")),
        None
    );
    assert_eq!(
        catalog.relation_named(&QualifiedName::new("public", "widget")),
        Some(h.widget)
    );
    assert_eq!(
        catalog.relation_named(&QualifiedName::new("pg_catalog", "widget")),
        None
    );
}

#[test]
fn overloads_come_back_in_oid_order() {
    let (catalog, h) = small_catalog();
    assert_eq!(
        catalog.functions_named(&QualifiedName::new("pg_catalog", "f")),
        &[h.catalog_f_int4, h.catalog_f_text]
    );
    assert_eq!(
        catalog.functions_named(&QualifiedName::new("public", "f")),
        &[h.public_f_int4, h.public_f_text]
    );
    assert!(
        catalog
            .functions_named(&QualifiedName::new("public", "g"))
            .is_empty()
    );
    assert!(
        catalog
            .functions_named(&QualifiedName::new("nope", "f"))
            .is_empty()
    );

    assert_eq!(
        catalog.operators_named(&QualifiedName::new("pg_catalog", "+")),
        &[h.plus]
    );
    assert!(
        catalog
            .operators_named(&QualifiedName::new("public", "+"))
            .is_empty()
    );
}

#[test]
fn the_search_path_adds_pg_catalog_first() {
    let (catalog, h) = small_catalog();
    let path = SearchPath::new(&catalog, ["public"]);
    assert_eq!(path.schemas(), &[h.pg_catalog, h.public]);
}

#[test]
fn an_explicit_pg_catalog_keeps_its_place() {
    let (catalog, h) = small_catalog();
    let path = SearchPath::new(&catalog, ["public", "pg_catalog"]);
    assert_eq!(path.schemas(), &[h.public, h.pg_catalog]);
}

#[test]
fn an_unknown_schema_leaves_the_search_path() {
    let (catalog, h) = small_catalog();
    let path = SearchPath::new(&catalog, ["nope", "public", "also_nope"]);
    assert_eq!(path.schemas(), &[h.pg_catalog, h.public]);
}

#[test]
fn the_search_path_comes_from_the_metadata() {
    let (catalog, h) = small_catalog();
    let path = SearchPath::from_metadata(&catalog);
    assert_eq!(path.schemas(), &[h.pg_catalog, h.public]);
}

#[test]
fn a_visible_type_is_the_first_one_on_the_path() {
    let (catalog, h) = small_catalog();
    let path = SearchPath::new(&catalog, ["public"]);
    assert_eq!(catalog.type_visible(&path, "int4"), Some(h.int4));
    assert_eq!(catalog.type_visible(&path, "mood"), Some(h.mood));
    assert_eq!(catalog.type_visible(&path, "nothing"), None);

    assert_eq!(catalog.relation_visible(&path, "widget"), Some(h.widget));
    assert_eq!(catalog.relation_visible(&path, "nothing"), None);
}

#[test]
fn visible_functions_follow_the_path_then_the_oid() {
    let (catalog, h) = small_catalog();

    let implicit = SearchPath::new(&catalog, ["public"]);
    assert_eq!(
        catalog.functions_visible(&implicit, "f"),
        vec![
            h.catalog_f_int4,
            h.catalog_f_text,
            h.public_f_int4,
            h.public_f_text
        ]
    );

    let explicit = SearchPath::new(&catalog, ["public", "pg_catalog"]);
    assert_eq!(
        catalog.functions_visible(&explicit, "f"),
        vec![
            h.public_f_int4,
            h.public_f_text,
            h.catalog_f_int4,
            h.catalog_f_text
        ]
    );
    assert!(catalog.functions_visible(&explicit, "g").is_empty());

    assert_eq!(catalog.operators_visible(&implicit, "+"), vec![h.plus]);
    assert!(catalog.operators_visible(&implicit, "-").is_empty());
}

#[test]
fn casts_answer_by_pair_and_by_source() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog.cast(h.int4, h.text), Some(h.int4_to_text));
    assert_eq!(catalog.cast(h.text, h.int4), None);
    // OID 50000 comes before OID 50001, although the pushes went the other
    // way round.
    assert_eq!(
        catalog.casts_from(h.int4),
        &[h.int4_to_mood, h.int4_to_text]
    );
    assert!(catalog.casts_from(h.text).is_empty());
}

#[test]
fn qualified_names_read_the_schema_of_the_entity() {
    let (catalog, h) = small_catalog();
    assert_eq!(
        catalog.qualified_name_of_type(h.int4).to_string(),
        "pg_catalog.int4"
    );
    assert_eq!(
        catalog.qualified_name_of_relation(h.widget).to_string(),
        "public.widget"
    );
    assert_eq!(
        catalog.qualified_name_of_function(h.public_f_int4),
        QualifiedName::new("public", "f")
    );
    assert_eq!(
        catalog.qualified_name_of_operator(h.plus),
        QualifiedName::new("pg_catalog", "+")
    );
}

#[test]
fn the_composite_and_its_relation_point_at_each_other() {
    let (catalog, h) = small_catalog();
    let TypeKind::Composite(detail) = &catalog[h.widget_row].kind else {
        panic!("the row type is a composite");
    };
    assert_eq!(detail.relation, Some(h.widget));
    assert_eq!(detail.fields.len(), 2);
    assert_eq!(catalog[h.widget].row_type, h.widget_row);
    assert_eq!(
        catalog[h.widget].column("label").map(|c| c.typmod),
        Some(Typmod(14))
    );
    assert!(catalog[h.widget].column("nothing").is_none());
}

#[test]
fn the_array_and_its_element_point_at_each_other() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog[h.int4].array, Some(h.int4_array));
    assert_eq!(
        catalog[h.int4_array].kind,
        TypeKind::Array { element: h.int4 }
    );
    assert_eq!(catalog[h.int4_array].array, None);
}

#[test]
fn the_domain_names_its_base_type() {
    let (catalog, h) = small_catalog();
    let TypeKind::Domain(detail) = &catalog[h.positive].kind else {
        panic!("positive_int is a domain");
    };
    assert_eq!(detail.base, h.int4);
    assert!(detail.not_null);
    assert_eq!(detail.check_constraints, 1);
}

#[test]
fn the_enum_keeps_its_labels_in_order() {
    let (catalog, h) = small_catalog();
    let TypeKind::Enum { labels } = &catalog[h.mood].kind else {
        panic!("mood is an enum");
    };
    assert_eq!(
        labels,
        &[Name::new("sad"), Name::new("ok"), Name::new("happy")]
    );
    assert_eq!(catalog[h.mood].extension, Some(h.widgets));
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    /// A catalog that holds every built-in type, so that it passes the
    /// check that a deserialize runs.
    fn full_catalog() -> Catalog {
        let mut builder = CatalogBuilder::new(Metadata::placeholder());
        let pg_catalog = builder.push_schema(Schema {
            oid: Oid(11),
            name: Name::new("pg_catalog"),
            extension: None,
        });
        let public = builder.push_schema(Schema {
            oid: Oid(2200),
            name: Name::new("public"),
            extension: None,
        });
        let mut text = None;
        for builtin in Builtin::ALL {
            let handle = builder.push_type(base_type(
                builtin.oid().0,
                builtin.name(),
                pg_catalog,
                TypeCategory::UserDefined,
            ));
            if *builtin == Builtin::Text {
                text = Some(handle);
            }
        }
        let text = text.expect("the loop pushed text");
        builder.push_function(function(30000, "f", public, text, text));
        builder.finish().expect("the catalog builds")
    }

    #[test]
    fn a_catalog_makes_a_round_trip() {
        let catalog = full_catalog();
        let text = serde_json::to_string(&catalog).expect("serializes");
        let back: Catalog = serde_json::from_str(&text).expect("deserializes");

        assert_eq!(back.metadata(), catalog.metadata());
        assert_eq!(back.types().len(), catalog.types().len());
        assert_eq!(back.schemas().len(), 2);
        assert_eq!(back.functions().len(), 1);
        assert_eq!(back.builtin(Builtin::Int4), catalog.builtin(Builtin::Int4));
        assert_eq!(
            back.type_named(&QualifiedName::new("pg_catalog", "int4")),
            Some(back.builtin(Builtin::Int4))
        );
        assert_eq!(
            back.functions_named(&QualifiedName::new("public", "f"))
                .len(),
            1
        );
    }

    #[test]
    fn a_handle_serializes_as_a_bare_number() {
        let catalog = full_catalog();
        let value: serde_json::Value =
            serde_json::to_value(&catalog).expect("serializes to a value");
        assert!(value["types"][0]["schema"].is_number());
    }

    #[test]
    fn another_format_version_is_rejected() {
        let catalog = full_catalog();
        let mut value: serde_json::Value =
            serde_json::to_value(&catalog).expect("serializes to a value");
        value["metadata"]["format_version"] = serde_json::json!(Catalog::FORMAT_VERSION + 1);
        let error = serde_json::from_value::<Catalog>(value).expect_err("is rejected");
        assert!(
            error.to_string().contains("format version"),
            "message was {error}"
        );
    }

    #[test]
    fn a_catalog_without_the_builtins_is_rejected() {
        let (catalog, _) = small_catalog();
        let text = serde_json::to_string(&catalog).expect("serializes");
        let error = serde_json::from_str::<Catalog>(&text).expect_err("is rejected");
        assert!(
            error.to_string().contains("is missing"),
            "message was {error}"
        );
    }
}
