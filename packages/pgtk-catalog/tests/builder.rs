//! Build a small catalog by hand, then exercise every lookup.

use pgtk_catalog::{
    ArgMode, Argument, BtreeOperators, BuildError, Builtin, Cast, CastContext, CastMethod, Catalog,
    CatalogBuilder, Collation, CollationProvider, Column, CompositeDetail, Constraint,
    ConstraintKind, DomainDetail, Encoding, Extension, Field, FiresWhen, Function, FunctionKind,
    Handle, Identity, Metadata, Name, Oid, Operator, OperatorClass, OperatorKind, Parameter,
    ParameterKind, PrimaryKey, QualifiedName, Relation, RelationKind, Rule, RuleEvent, Schema,
    SearchPath, Trigger, TriggerTiming, Type, TypeCategory, TypeKind, Typmod, Volatility,
};

// These checks run when the test compiles, not when it runs.
//
// A catalog is `Send` and `Sync`, so a consumer shares one behind an `Arc`.
// A handle is `Copy`, `Send`, and `Sync` whatever its marker type is, even a
// marker type that is neither `Send` nor `Sync`, because a handle holds
// `PhantomData<fn() -> T>` and never a `T`.
const _: () = {
    /// A type that is neither `Send` nor `Sync`.
    type NotSend = std::rc::Rc<()>;

    const fn assert_shared<T: Send + Sync>() {}
    const fn assert_copy_shared<T: Copy + Send + Sync>() {}

    assert_shared::<Catalog>();
    assert_copy_shared::<Handle<NotSend>>();
    assert_copy_shared::<Handle<Type>>();
    assert_copy_shared::<Handle<Schema>>();
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
        collation: None,
        array: None,
        kind: TypeKind::Base,
        btree: None,
        hash_equality: None,
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
        default_types: Vec::new(),
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
        is_temporary: false,
        extension: None,
    });
    let public = builder.push_schema(Schema {
        oid: Oid(2200),
        name: Name::new("public"),
        is_temporary: false,
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
        row_type: Some(widget_row),
        columns: vec![
            Column {
                name: Name::new("id"),
                position: 1,
                ty: int4,
                typmod: Typmod::NONE,
                not_null: true,
                has_default: false,
                generated: false,
                collation: None,
                identity: Some(Identity::Always),
            },
            Column {
                name: Name::new("label"),
                position: 2,
                ty: text,
                typmod: Typmod(14),
                not_null: false,
                has_default: true,
                generated: false,
                collation: None,
                identity: None,
            },
        ],
        of_type: None,
        parents: Vec::new(),
        primary_key: None,
        constraints: Vec::new(),
        view_definition: None,
        rules: Vec::new(),
        triggers: Vec::new(),
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

/// The handles of the entities that [`temporary_catalog`] builds.
struct TemporaryHandles {
    pg_catalog: Handle<Schema>,
    public: Handle<Schema>,
    temporary: Handle<Schema>,
    public_widget: Handle<Relation>,
    temporary_widget: Handle<Relation>,
    public_f: Handle<Function>,
    temporary_f: Handle<Function>,
}

/// A catalog with a temporary schema. A relation `widget` and a function `f`
/// are in both `public` and the temporary schema.
fn temporary_catalog() -> (Catalog, TemporaryHandles) {
    let mut builder = CatalogBuilder::new(Metadata::placeholder());
    builder.permit_missing_builtins();

    let schema = |oid: u32, name: &str, is_temporary: bool| Schema {
        oid: Oid(oid),
        name: Name::new(name),
        is_temporary,
        extension: None,
    };
    let pg_catalog = builder.push_schema(schema(11, "pg_catalog", false));
    let public = builder.push_schema(schema(2200, "public", false));
    let temporary = builder.push_schema(schema(16500, "pg_temp_3", true));
    let int4 = builder.push_type(base_type(23, "int4", pg_catalog, TypeCategory::Numeric));

    // A table with no column, as `CREATE TEMP TABLE nocols()` makes.
    let mut widget = |oid: u32, schema: Handle<Schema>| {
        let row_type = builder.next_type_handle();
        let relation = builder.next_relation_handle();
        builder.push_type(Type {
            kind: TypeKind::Composite(CompositeDetail {
                fields: Vec::new(),
                relation: Some(relation),
            }),
            ..base_type(oid + 1, "widget", schema, TypeCategory::Composite)
        });
        builder.push_relation(Relation {
            oid: Oid(oid),
            name: Name::new("widget"),
            schema,
            kind: RelationKind::Table,
            row_type: Some(row_type),
            columns: Vec::new(),
            of_type: None,
            parents: Vec::new(),
            primary_key: None,
            constraints: Vec::new(),
            view_definition: None,
            rules: Vec::new(),
            triggers: Vec::new(),
            extension: None,
        })
    };
    let public_widget = widget(20000, public);
    let temporary_widget = widget(20010, temporary);

    let public_f = builder.push_function(function(30000, "f", public, int4, int4));
    let temporary_f = builder.push_function(function(30001, "f", temporary, int4, int4));

    let catalog = builder.finish().expect("the catalog builds");
    (
        catalog,
        TemporaryHandles {
            pg_catalog,
            public,
            temporary,
            public_widget,
            temporary_widget,
            public_f,
            temporary_f,
        },
    )
}

#[test]
fn a_catalog_names_its_temporary_schema() {
    let (catalog, h) = temporary_catalog();
    assert_eq!(catalog.temporary_schema(), Some(h.temporary));
    assert!(catalog[h.temporary].is_temporary);
    assert!(!catalog[h.public].is_temporary);

    let (catalog, _) = small_catalog();
    assert_eq!(catalog.temporary_schema(), None);
}

#[test]
fn the_search_path_adds_the_temporary_schema_before_pg_catalog() {
    let (catalog, h) = temporary_catalog();
    let path = SearchPath::new(&catalog, ["public"]);
    assert_eq!(path.schemas(), &[h.temporary, h.pg_catalog, h.public]);
    assert_eq!(
        catalog.relation_visible(&path, "widget"),
        Some(h.temporary_widget)
    );
}

#[test]
fn an_explicit_pg_temp_keeps_its_place() {
    let (catalog, h) = temporary_catalog();
    let path = SearchPath::new(&catalog, ["public", "pg_temp"]);
    assert_eq!(path.schemas(), &[h.pg_catalog, h.public, h.temporary]);
    assert_eq!(
        catalog.relation_visible(&path, "widget"),
        Some(h.public_widget)
    );

    // The name that the server gives the schema does the same.
    let concrete = SearchPath::new(&catalog, ["public", "pg_temp_3"]);
    assert_eq!(concrete, path);

    // The two names together are one schema, at its first place.
    let both = SearchPath::new(&catalog, ["pg_temp", "public", "pg_temp_3"]);
    assert_eq!(both.schemas(), &[h.pg_catalog, h.temporary, h.public]);
}

#[test]
fn pg_temp_matches_nothing_without_a_temporary_schema() {
    let (catalog, h) = small_catalog();
    let path = SearchPath::new(&catalog, ["pg_temp", "public"]);
    assert_eq!(path.schemas(), &[h.pg_catalog, h.public]);
}

#[test]
fn a_function_lookup_skips_the_temporary_schema() {
    let (catalog, h) = temporary_catalog();
    let path = SearchPath::new(&catalog, ["pg_temp", "public"]);
    assert_eq!(catalog.functions_visible(&path, "f"), vec![h.public_f]);
    assert_eq!(
        catalog.functions_named(&QualifiedName::new("pg_temp_3", "f")),
        &[h.temporary_f]
    );
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
fn one_signature_in_two_schemas_gives_two_candidates() {
    // `pg_catalog.f(int4)` and `public.f(int4)` have the same signature.
    // PostgreSQL hides the later one. The catalog applies no hiding rule, so
    // both come back and the consumer decides.
    let (catalog, h) = small_catalog();
    let path = SearchPath::new(&catalog, ["public"]);

    let signature = |handle: &Handle<Function>| {
        let function = &catalog[*handle];
        let arguments: Vec<Handle<Type>> = function.arguments.iter().map(|a| a.ty).collect();
        (function.schema, arguments)
    };

    let visible = catalog.functions_visible(&path, "f");
    let twins: Vec<&Handle<Function>> = visible
        .iter()
        .filter(|handle| signature(handle).1 == vec![h.int4])
        .collect();
    assert_eq!(
        twins,
        vec![&h.catalog_f_int4, &h.public_f_int4],
        "both f(int4) overloads are candidates"
    );
    assert_eq!(catalog[*twins[0]].schema, h.pg_catalog);
    assert_eq!(catalog[*twins[1]].schema, h.public);
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
    assert_eq!(catalog[h.widget].row_type, Some(h.widget_row));
    assert_eq!(
        catalog[h.widget].column("label").map(|c| c.typmod),
        Some(Typmod(14))
    );
    assert!(catalog[h.widget].column("nothing").is_none());
}

/// A column of a sequence. Each one is NOT NULL and has no default.
fn sequence_column(name: &str, position: i16, ty: Handle<Type>) -> Column {
    Column {
        name: Name::new(name),
        position,
        ty,
        typmod: Typmod::NONE,
        not_null: true,
        has_default: false,
        generated: false,
        collation: None,
        identity: None,
    }
}

#[test]
fn a_sequence_is_a_relation_with_three_columns_and_no_row_type() {
    let mut builder = CatalogBuilder::new(Metadata {
        search_path: vec![Name::new("public")],
        ..Metadata::placeholder()
    });
    builder.permit_missing_builtins();

    let pg_catalog = builder.push_schema(Schema {
        oid: Oid(11),
        name: Name::new("pg_catalog"),
        is_temporary: false,
        extension: None,
    });
    let public = builder.push_schema(Schema {
        oid: Oid(2200),
        name: Name::new("public"),
        is_temporary: false,
        extension: None,
    });
    let boolean = builder.push_type(base_type(16, "bool", pg_catalog, TypeCategory::Boolean));
    let int8 = builder.push_type(base_type(20, "int8", pg_catalog, TypeCategory::Numeric));

    // A sequence has no row type, so no composite goes in beside it.
    let sequence = builder.next_relation_handle();
    builder.push_relation(Relation {
        oid: Oid(20010),
        name: Name::new("widget_id_seq"),
        schema: public,
        kind: RelationKind::Sequence,
        row_type: None,
        columns: vec![
            sequence_column("last_value", 1, int8),
            sequence_column("log_cnt", 2, int8),
            sequence_column("is_called", 3, boolean),
        ],
        of_type: None,
        parents: Vec::new(),
        primary_key: None,
        constraints: Vec::new(),
        view_definition: None,
        rules: Vec::new(),
        triggers: Vec::new(),
        extension: None,
    });
    let catalog = builder.finish().expect("the catalog builds");

    assert_eq!(
        catalog.relation_named(&QualifiedName::new("public", "widget_id_seq")),
        Some(sequence)
    );
    let path = SearchPath::new(&catalog, ["public"]);
    assert_eq!(
        catalog.relation_visible(&path, "widget_id_seq"),
        Some(sequence)
    );

    let relation = &catalog[sequence];
    assert_eq!(relation.kind, RelationKind::Sequence);
    assert_eq!(relation.row_type, None);
    let columns: Vec<_> = relation
        .columns
        .iter()
        .map(|column| (column.position, column.name.as_str(), column.ty))
        .collect();
    assert_eq!(
        columns,
        vec![
            (1, "last_value", int8),
            (2, "log_cnt", int8),
            (3, "is_called", boolean),
        ]
    );
}

#[test]
fn a_type_answers_with_its_btree_operators_and_its_hash_equality() {
    let (catalog, h) = small_catalog();
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();

    // One comparison operator for `int4`, for each of the five symbols.
    let boolean = builder.push_type(base_type(16, "bool", h.pg_catalog, TypeCategory::Boolean));
    let mut comparison = |oid: u32, symbol: &str| {
        builder.push_operator(Operator {
            oid: Oid(oid),
            name: Name::new(symbol),
            schema: h.pg_catalog,
            kind: OperatorKind::Infix,
            left: Some(h.int4),
            right: h.int4,
            result: boolean,
            function: h.catalog_f_int4,
            commutator: None,
            negator: None,
            extension: None,
        })
    };
    let btree = BtreeOperators {
        lt: comparison(97, "<"),
        le: comparison(523, "<="),
        eq: comparison(96, "="),
        ge: comparison(525, ">="),
        gt: comparison(521, ">"),
    };
    builder.type_mut(h.int4).btree = Some(btree);
    builder.type_mut(h.int4).hash_equality = Some(btree.eq);
    let catalog = builder.finish().expect("the catalog builds");

    assert_eq!(catalog.btree_ordering(h.int4), Some(btree));
    assert_eq!(catalog.hash_equality(h.int4), Some(btree.eq));
    assert_eq!(catalog[btree.lt].name, Name::new("<"));
    assert_eq!(catalog[btree.gt].name, Name::new(">"));

    // A type with no operator class, as `json` and `xml` are on a server.
    assert_eq!(catalog.btree_ordering(h.text), None);
    assert_eq!(catalog.hash_equality(h.text), None);
}

#[test]
fn a_relation_keeps_its_primary_key_in_key_order() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog[h.widget].primary_key, None);

    // The key is `(label, id)`, which is not the order of the columns.
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    builder.relation_mut(h.widget).primary_key = Some(PrimaryKey {
        columns: vec![2, 1],
        deferrable: false,
    });
    let catalog = builder.finish().expect("the catalog builds");

    let relation = &catalog[h.widget];
    let key = relation.primary_key.as_ref().expect("widget has a key");
    assert_eq!(key.grouping_columns(), Some(&[2, 1][..]));
    let names: Vec<&str> = key
        .columns
        .iter()
        .map(|position| relation.column_at(*position).unwrap().name.as_str())
        .collect();
    assert_eq!(names, vec!["label", "id"]);
    assert!(relation.column_at(3).is_none());
}

#[test]
fn a_deferrable_primary_key_proves_no_grouping() {
    let key = PrimaryKey {
        columns: vec![1],
        deferrable: true,
    };
    assert_eq!(key.grouping_columns(), None);
}

#[test]
fn a_primary_key_on_an_unknown_column_is_rejected() {
    let (catalog, h) = small_catalog();
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    builder.relation_mut(h.widget).primary_key = Some(PrimaryKey {
        columns: vec![1, 9],
        deferrable: false,
    });
    assert!(matches!(
        builder.finish(),
        Err(BuildError::UnknownKeyColumn {
            relation: Oid(20003),
            position: 9,
        })
    ));
}

#[test]
fn a_column_keeps_the_kind_of_its_identity() {
    let (catalog, h) = small_catalog();
    let widget = &catalog[h.widget];
    assert_eq!(
        widget.column("id").map(|c| c.identity),
        Some(Some(Identity::Always))
    );
    assert_eq!(widget.column("label").map(|c| c.identity), Some(None));
    assert_ne!(Identity::Always, Identity::ByDefault);
}

#[test]
fn a_relation_names_its_direct_parents_in_order() {
    let (catalog, h) = temporary_catalog();
    assert_eq!(catalog[h.public_widget].parents, Vec::new());

    // The temporary `widget` inherits from the `widget` in `public`.
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    builder.relation_mut(h.temporary_widget).parents = vec![h.public_widget];
    let catalog = builder.finish().expect("the catalog builds");
    assert_eq!(catalog[h.temporary_widget].parents, vec![h.public_widget]);
}

#[test]
fn a_function_answers_with_the_type_of_a_default() {
    let (catalog, h) = small_catalog();
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();

    // `g(a int4, b text = ..., c text = ...)`. The default of `b` is a
    // `text` expression. The loader cannot tell the type of the default of
    // `c`.
    let argument = |name: &str, ty| Argument {
        name: Some(Name::new(name)),
        ty,
        mode: ArgMode::In,
    };
    let g = builder.push_function(Function {
        arguments: vec![
            argument("a", h.int4),
            argument("b", h.text),
            argument("c", h.text),
        ],
        default_count: 2,
        default_types: vec![Some(h.text), None],
        ..function(30010, "g", h.public, h.int4, h.int4)
    });
    let catalog = builder.finish().expect("the catalog builds");

    let g = &catalog[g];
    assert_eq!(g.default_type_of(0), None);
    assert_eq!(g.default_type_of(1), Some(h.text));
    assert_eq!(g.default_type_of(2), None);
    assert_eq!(g.default_type_of(3), None);
}

#[test]
fn a_function_needs_one_default_type_for_each_default() {
    let (catalog, h) = small_catalog();
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    builder.push_function(Function {
        default_count: 1,
        default_types: Vec::new(),
        ..function(30010, "g", h.public, h.int4, h.int4)
    });
    assert!(matches!(
        builder.finish(),
        Err(BuildError::DefaultTypesLength {
            function: Oid(30010),
            default_count: 1,
            default_types: 0,
        })
    ));
}

#[test]
fn a_typed_table_names_the_type_that_it_is_of() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog[h.widget].of_type, None);

    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    let widget_type = builder.push_type(Type {
        kind: TypeKind::Composite(CompositeDetail {
            fields: Vec::new(),
            relation: None,
        }),
        ..base_type(20020, "widget_type", h.public, TypeCategory::Composite)
    });
    builder.relation_mut(h.widget).of_type = Some(widget_type);
    let catalog = builder.finish().expect("the catalog builds");

    // The type of the table and its row type are two types.
    assert_eq!(catalog[h.widget].of_type, Some(widget_type));
    assert_eq!(catalog[h.widget].row_type, Some(h.widget_row));
}

#[test]
fn a_view_keeps_its_text_its_rules_and_its_triggers() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog[h.widget].view_definition, None);
    assert_eq!(catalog[h.widget].rules, Vec::new());
    assert_eq!(catalog[h.widget].triggers, Vec::new());

    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    let widget = builder.relation_mut(h.widget);
    widget.kind = RelationKind::View;
    widget.view_definition = Some(" SELECT 1 AS id, 'a'::text AS label;".to_owned());
    widget.rules = vec![Rule {
        name: Name::new("widget_ins"),
        event: RuleEvent::Insert,
        instead: true,
        unconditional: true,
        fires: FiresWhen::Origin,
        definition: String::new(),
    }];
    // INSTEAD OF UPDATE FOR EACH ROW: 64 + 16 + 1.
    widget.triggers = vec![Trigger::from_tgtype(
        Name::new("widget_upd"),
        81,
        FiresWhen::Origin,
    )];
    let catalog = builder.finish().expect("the catalog builds");

    let widget = &catalog[h.widget];
    assert!(
        widget
            .view_definition
            .as_deref()
            .unwrap()
            .contains("SELECT")
    );
    assert!(widget.rules[0].replaces(RuleEvent::Insert));
    assert!(!widget.rules[0].replaces(RuleEvent::Delete));
    let trigger = &widget.triggers[0];
    assert_eq!(trigger.timing, TriggerTiming::InsteadOf);
    assert!(trigger.update && !trigger.insert && !trigger.delete && !trigger.truncate);
    assert!(trigger.is_instead_of_row());
}

#[test]
fn a_disabled_or_conditional_rule_does_not_replace_a_statement() {
    let rule = |unconditional, instead, fires| Rule {
        name: Name::new("r"),
        event: RuleEvent::Delete,
        instead,
        unconditional,
        fires,
        definition: String::new(),
    };
    assert!(rule(true, true, FiresWhen::Origin).replaces(RuleEvent::Delete));
    assert!(rule(true, true, FiresWhen::Always).replaces(RuleEvent::Delete));
    assert!(!rule(false, true, FiresWhen::Origin).replaces(RuleEvent::Delete));
    assert!(!rule(true, false, FiresWhen::Origin).replaces(RuleEvent::Delete));
    assert!(!rule(true, true, FiresWhen::Disabled).replaces(RuleEvent::Delete));
    assert!(!rule(true, true, FiresWhen::Replica).replaces(RuleEvent::Delete));
}

/// A catalog that holds only these parameters.
fn catalog_with_parameters(parameters: &[(&str, ParameterKind)]) -> Result<Catalog, BuildError> {
    let mut builder = CatalogBuilder::new(Metadata::placeholder());
    builder.permit_missing_builtins();
    for (name, kind) in parameters {
        builder.push_parameter(Parameter {
            name: (*name).to_owned(),
            kind: *kind,
        });
    }
    builder.finish()
}

#[test]
fn a_parameter_answers_a_name_in_any_case_with_its_canonical_spelling() {
    let catalog = catalog_with_parameters(&[
        ("work_mem", ParameterKind::Integer),
        ("TimeZone", ParameterKind::String),
        ("DateStyle", ParameterKind::String),
    ])
    .expect("the catalog builds");

    for spelling in ["timezone", "TIMEZONE", "TimeZone"] {
        let parameter = catalog.parameter(spelling).expect("the parameter is there");
        assert_eq!(parameter.name, "TimeZone");
        assert_eq!(parameter.kind, ParameterKind::String);
    }
    assert_eq!(catalog.parameter("nope"), None);

    // The order is that of the lower-case name.
    let names: Vec<&str> = catalog
        .parameters()
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
    assert_eq!(names, vec!["DateStyle", "TimeZone", "work_mem"]);
}

#[test]
fn two_parameters_with_one_name_are_rejected() {
    let error = catalog_with_parameters(&[
        ("TimeZone", ParameterKind::String),
        ("timezone", ParameterKind::String),
    ])
    .unwrap_err();
    assert!(matches!(
        error,
        BuildError::DuplicateParameter { name } if name == "timezone"
    ));
}

#[test]
fn a_catalog_keeps_its_parameters_through_a_builder() {
    let catalog =
        catalog_with_parameters(&[("TimeZone", ParameterKind::String)]).expect("it builds");
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    let catalog = builder.finish().expect("the catalog builds");
    assert_eq!(catalog.parameters().len(), 1);
}

/// The handles of the collations that [`collation_catalog`] builds.
struct CollationHandles {
    default: Handle<Collation>,
    public_fr_latin1: Handle<Collation>,
    catalog_fr_utf8: Handle<Collation>,
    catalog_fr_latin1: Handle<Collation>,
    icu_und: Handle<Collation>,
}

/// A catalog of a database with this encoding. `pg_catalog` has `default`,
/// `fr` for UTF8 and for LATIN1, and the ICU collation `und-x-icu`. `public`
/// has `fr` for LATIN1 only.
fn collation_catalog(encoding: Encoding) -> (Catalog, CollationHandles) {
    let mut builder = CatalogBuilder::new(Metadata {
        encoding,
        ..Metadata::placeholder()
    });
    builder.permit_missing_builtins();
    let schema = |oid: u32, name: &str| Schema {
        oid: Oid(oid),
        name: Name::new(name),
        is_temporary: false,
        extension: None,
    };
    let pg_catalog = builder.push_schema(schema(11, "pg_catalog"));
    let public = builder.push_schema(schema(2200, "public"));

    let mut collation = |oid: u32, name: &str, schema, encoding, provider| {
        builder.push_collation(Collation {
            oid: Oid(oid),
            name: Name::new(name),
            schema,
            encoding,
            provider,
            deterministic: true,
            extension: None,
        })
    };
    let handles = CollationHandles {
        default: collation(100, "default", pg_catalog, None, CollationProvider::Default),
        public_fr_latin1: collation(20000, "fr", public, Some(8), CollationProvider::Libc),
        catalog_fr_utf8: collation(12001, "fr", pg_catalog, Some(6), CollationProvider::Libc),
        catalog_fr_latin1: collation(12002, "fr", pg_catalog, Some(8), CollationProvider::Libc),
        icu_und: collation(12003, "und-x-icu", pg_catalog, None, CollationProvider::Icu),
    };
    (builder.finish().expect("the catalog builds"), handles)
}

#[test]
fn a_collation_lookup_reads_the_encoding_of_the_database() {
    let name = QualifiedName::new("pg_catalog", "fr");

    let (utf8, h) = collation_catalog(Encoding::utf8());
    assert_eq!(utf8.collation_named(&name), Some(h.catalog_fr_utf8));
    assert_eq!(
        utf8.collations_named(&name),
        &[h.catalog_fr_utf8, h.catalog_fr_latin1]
    );

    let latin1 = Encoding {
        id: 8,
        name: "LATIN1".to_owned(),
    };
    let (latin1, h) = collation_catalog(latin1);
    assert_eq!(latin1.collation_named(&name), Some(h.catalog_fr_latin1));

    // `public.fr` is for LATIN1 only. A UTF8 database does not find it, but
    // the raw lookup shows that the name is there for another encoding.
    let public_fr = QualifiedName::new("public", "fr");
    assert_eq!(utf8.collation_named(&public_fr), None);
    assert_eq!(utf8.collations_named(&public_fr).len(), 1);
    assert_eq!(latin1.collation_named(&public_fr), Some(h.public_fr_latin1));

    // A collation for each encoding is a match in each database.
    let default = QualifiedName::new("pg_catalog", "default");
    assert_eq!(utf8.collation_named(&default), Some(h.default));
    assert_eq!(latin1.collation_named(&default), Some(h.default));
    assert_eq!(
        utf8.collation_named(&QualifiedName::new("pg_catalog", "nope")),
        None
    );
}

#[test]
fn a_visible_collation_is_the_first_usable_one_on_the_path() {
    // `public` is before `pg_catalog`, and it has `fr` only for LATIN1. A
    // UTF8 database goes on to `pg_catalog`.
    let (utf8, h) = collation_catalog(Encoding::utf8());
    let path = SearchPath::new(&utf8, ["public", "pg_catalog"]);
    assert_eq!(utf8.collation_visible(&path, "fr"), Some(h.catalog_fr_utf8));
    assert_eq!(utf8.collation_visible(&path, "default"), Some(h.default));
    assert_eq!(utf8.collation_visible(&path, "nope"), None);

    let latin1 = Encoding {
        id: 8,
        name: "LATIN1".to_owned(),
    };
    let (latin1, h) = collation_catalog(latin1);
    let path = SearchPath::new(&latin1, ["public", "pg_catalog"]);
    assert_eq!(
        latin1.collation_visible(&path, "fr"),
        Some(h.public_fr_latin1)
    );
}

#[test]
fn an_icu_collation_needs_an_encoding_that_icu_can_work_with() {
    let name = QualifiedName::new("pg_catalog", "und-x-icu");
    let (utf8, h) = collation_catalog(Encoding::utf8());
    assert_eq!(utf8.collation_named(&name), Some(h.icu_und));

    let sql_ascii = Encoding {
        id: 0,
        name: "SQL_ASCII".to_owned(),
    };
    assert!(!sql_ascii.is_supported_by_icu());
    let (ascii, h) = collation_catalog(sql_ascii);
    assert_eq!(ascii.collation_named(&name), None);
    // The `default` collation is not an ICU collation, so it is still there.
    assert_eq!(
        ascii.collation_named(&QualifiedName::new("pg_catalog", "default")),
        Some(h.default)
    );
}

#[test]
fn a_collation_answers_its_oid_and_its_qualified_name() {
    let (catalog, h) = collation_catalog(Encoding::utf8());
    assert_eq!(catalog.collations().len(), 5);
    assert_eq!(catalog.collation_by_oid(Oid(100)), Some(h.default));
    assert_eq!(
        catalog.qualified_name_of_collation(h.icu_und),
        QualifiedName::new("pg_catalog", "und-x-icu")
    );
    // The arena is in push order for a hand-built catalog.
    assert_eq!(catalog[h.public_fr_latin1].encoding, Some(8));
}

#[test]
fn a_type_and_a_column_point_at_a_collation() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog[h.text].collation, None);

    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    let default = builder.push_collation(Collation {
        oid: Oid(100),
        name: Name::new("default"),
        schema: h.pg_catalog,
        encoding: None,
        provider: CollationProvider::Default,
        deterministic: true,
        extension: None,
    });
    builder.type_mut(h.text).is_collatable = true;
    builder.type_mut(h.text).collation = Some(default);
    builder.relation_mut(h.widget).columns[1].collation = Some(default);
    let catalog = builder.finish().expect("the catalog builds");

    assert_eq!(catalog[h.text].collation, Some(default));
    assert_eq!(catalog[h.int4].collation, None);
    let widget = &catalog[h.widget];
    assert_eq!(widget.column("label").unwrap().collation, Some(default));
    assert_eq!(widget.column("id").unwrap().collation, None);
}

#[test]
fn a_relation_answers_a_constraint_name() {
    let (catalog, h) = small_catalog();
    assert_eq!(catalog[h.widget].constraints, Vec::new());

    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    builder.relation_mut(h.widget).constraints = vec![
        Constraint {
            name: Name::new("widget_label_key"),
            kind: ConstraintKind::Unique,
            deferrable: false,
            columns: vec![2],
        },
        Constraint {
            name: Name::new("widget_id_check"),
            kind: ConstraintKind::Check,
            deferrable: false,
            columns: vec![1],
        },
    ];
    let catalog = builder.finish().expect("the catalog builds");

    let widget = &catalog[h.widget];
    let unique = widget.constraint("widget_label_key").expect("it is there");
    assert_eq!(unique.kind, ConstraintKind::Unique);
    assert!(unique.kind.has_index());
    assert!(!ConstraintKind::Check.has_index());
    assert!(!ConstraintKind::ForeignKey.has_index());
    assert!(widget.constraint("no_such_constraint").is_none());
}

#[test]
fn an_operator_class_answers_a_name_and_an_access_method() {
    let (catalog, h) = small_catalog();
    let mut builder = CatalogBuilder::from_catalog(catalog);
    builder.permit_missing_builtins();
    let mut class = |oid: u32, schema, method: &str, is_default| {
        builder.push_operator_class(OperatorClass {
            oid: Oid(oid),
            name: Name::new("int4_ops"),
            schema,
            method: Name::new(method),
            input_type: h.int4,
            is_default,
            extension: None,
        })
    };
    let btree = class(1978, h.pg_catalog, "btree", true);
    let hash = class(1977, h.pg_catalog, "hash", true);
    let public_btree = class(70000, h.public, "btree", false);
    let catalog = builder.finish().expect("the catalog builds");

    let name = QualifiedName::new("pg_catalog", "int4_ops");
    assert_eq!(catalog.operator_class_named(&name, "btree"), Some(btree));
    assert_eq!(catalog.operator_class_named(&name, "hash"), Some(hash));
    assert_eq!(catalog.operator_class_named(&name, "gist"), None);
    assert_eq!(
        catalog.operator_class_named(&QualifiedName::new("nope", "int4_ops"), "btree"),
        None
    );

    // `public` is after the implicit `pg_catalog` on this path.
    let path = SearchPath::new(&catalog, ["public"]);
    assert_eq!(
        catalog.operator_class_visible(&path, "int4_ops", "btree"),
        Some(btree)
    );
    let path = SearchPath::new(&catalog, ["public", "pg_catalog"]);
    assert_eq!(
        catalog.operator_class_visible(&path, "int4_ops", "btree"),
        Some(public_btree)
    );
    // `public` has no `hash` class with the name, so the walk goes on.
    assert_eq!(
        catalog.operator_class_visible(&path, "int4_ops", "hash"),
        Some(hash)
    );

    assert_eq!(catalog.operator_classes().len(), 3);
    assert_eq!(catalog.operator_class_by_oid(Oid(1977)), Some(hash));
    assert_eq!(
        catalog.qualified_name_of_operator_class(public_btree),
        QualifiedName::new("public", "int4_ops")
    );
    assert_eq!(catalog[btree].input_type, h.int4);
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
            is_temporary: false,
            extension: None,
        });
        let public = builder.push_schema(Schema {
            oid: Oid(2200),
            name: Name::new("public"),
            is_temporary: false,
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
