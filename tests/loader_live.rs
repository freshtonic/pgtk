//! Live tests against a real PostgreSQL server.
//!
//! The tests read the connection string from `PG_CATALOG_TEST_URL`. Each one
//! prints a message and passes when that variable is not set, so a checkout
//! with no server still runs `cargo test`.
//!
//! ```sh
//! PG_CATALOG_TEST_URL=postgres://postgres@localhost:5432/postgres \
//!     cargo test --features postgres --test loader_live
//! ```
//!
//! The file needs the `postgres` feature, the `tokio-postgres` feature, or
//! both. Without them it holds no test.

#![cfg(any(feature = "postgres", feature = "tokio-postgres"))]

/// The variable that holds the connection string.
const URL_VARIABLE: &str = "PG_CATALOG_TEST_URL";

/// The connection string, when the caller set it.
fn url() -> Option<String> {
    std::env::var(URL_VARIABLE).ok()
}

/// Print why a test did no work.
fn skip(test: &str) {
    println!("skipping {test}: set {URL_VARIABLE} to run it against a server");
}

/// The lock that lets one live test at a time use the server.
///
/// The tests share one database. One of them makes a schema and drops it
/// again, so a test beside it sees the database half made. Each test holds
/// this lock for the whole of its body.
static SERVER: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take the lock on the server.
///
/// A test that fails poisons the lock. The next test still takes it, because
/// the lock guards the server and not a value in memory.
fn lock_server() -> std::sync::MutexGuard<'static, ()> {
    SERVER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(feature = "postgres")]
mod sync_tests {
    use super::{lock_server, skip, url};
    use pg_catalog::{
        ArgMode, Builtin, CastMethod, Catalog, FunctionKind, Loaded, Name, QualifiedName,
        RelationKind, SearchPath, TypeKind,
    };
    use postgres::{Client, NoTls};
    use std::panic::AssertUnwindSafe;
    use std::time::Instant;

    /// Connect, load, and report how long the load took.
    fn load(client: &mut Client) -> Loaded {
        let start = Instant::now();
        let loaded = pg_catalog::load(client).expect("the load succeeds");
        println!(
            "loaded {} types, {} relations, {} functions, {} operators, {} casts in {:?}",
            loaded.catalog.types().len(),
            loaded.catalog.relations().len(),
            loaded.catalog.functions().len(),
            loaded.catalog.operators().len(),
            loaded.catalog.casts().len(),
            start.elapsed(),
        );
        loaded
    }

    /// The handle of one operator with these two operand types.
    fn infix_operator(
        catalog: &Catalog,
        name: &QualifiedName,
        left: &str,
        right: &str,
    ) -> Option<pg_catalog::Handle<pg_catalog::Operator>> {
        catalog
            .operators_named(name)
            .iter()
            .copied()
            .find(|handle| {
                let operator = &catalog[*handle];
                let type_name = |handle| catalog.qualified_name_of_type(handle).name;
                operator.left.map(type_name) == Some(Name::new(left))
                    && type_name(operator.right) == Name::new(right)
            })
    }

    #[test]
    fn it_loads_a_stock_database_with_no_diagnostics() {
        let Some(url) = url() else {
            skip("it_loads_a_stock_database_with_no_diagnostics");
            return;
        };
        let _guard = lock_server();
        let mut client = Client::connect(&url, NoTls).expect("the server accepts the connection");
        let loaded = load(&mut client);
        let catalog = &loaded.catalog;

        assert_eq!(loaded.diagnostics, Vec::new(), "the load has no diagnostic");

        let version = catalog.metadata().server_version;
        assert!(
            (14..=17).contains(&version.major),
            "version {version} is in range"
        );
        assert!(!catalog.metadata().search_path.is_empty());

        // `int4` is the built-in of the same name.
        let int4 = catalog.builtin(Builtin::Int4);
        assert_eq!(catalog[int4].name, Name::new("int4"));
        assert_eq!(catalog.as_builtin(int4), Some(Builtin::Int4));
        assert_eq!(
            catalog.qualified_name_of_type(int4),
            QualifiedName::new("pg_catalog", "int4")
        );

        // `int4[]` is the array of `int4`, and each points at the other.
        let int4_array = catalog.builtin(Builtin::Int4Array);
        assert_eq!(catalog[int4].array, Some(int4_array));
        assert_eq!(catalog[int4_array].kind, TypeKind::Array { element: int4 });
        assert_eq!(catalog[int4_array].array, None);

        // `+` for two `int4` gives an `int4`.
        let plus = QualifiedName::new("pg_catalog", "+");
        let handle =
            infix_operator(catalog, &plus, "int4", "int4").expect("int4 + int4 is an operator");
        let operator = &catalog[handle];
        assert_eq!(operator.result, int4);
        assert_eq!(
            catalog.qualified_name_of_function(operator.function).name,
            Name::new("int4pl")
        );

        // `text` to `varchar` is a binary cast.
        let text = catalog.builtin(Builtin::Text);
        let varchar = catalog.builtin(Builtin::Varchar);
        let cast = catalog.cast(text, varchar).expect("text casts to varchar");
        assert_eq!(catalog[cast].method, CastMethod::Binary);

        // `count` is an aggregate.
        let count = QualifiedName::new("pg_catalog", "count");
        let overloads = catalog.functions_named(&count);
        assert!(!overloads.is_empty(), "count has at least one overload");
        for handle in overloads {
            let function = &catalog[*handle];
            assert_eq!(function.kind, FunctionKind::Aggregate);
            assert!(function.aggregate.is_some(), "count has aggregate detail");
        }

        // `pg_class` is a table with a `relname` column.
        let pg_class = catalog
            .relation_named(&QualifiedName::new("pg_catalog", "pg_class"))
            .expect("pg_class is a relation");
        let relation = &catalog[pg_class];
        assert_eq!(relation.kind, RelationKind::Table);
        let relname = relation.column("relname").expect("pg_class has relname");
        assert_eq!(
            catalog.qualified_name_of_type(relname.ty).name,
            Name::new("name")
        );
        // The row type of pg_class names the relation back.
        let row_type = relation.row_type.expect("a table has a row type");
        let TypeKind::Composite(detail) = &catalog[row_type].kind else {
            panic!("the row type of a relation is a composite");
        };
        assert_eq!(detail.relation, Some(pg_class));
        assert_eq!(detail.fields.len(), relation.columns.len());
    }

    #[test]
    fn it_loads_custom_entities_with_the_right_shape() {
        let Some(url) = url() else {
            skip("it_loads_custom_entities_with_the_right_shape");
            return;
        };
        let _guard = lock_server();
        let mut client = Client::connect(&url, NoTls).expect("the server accepts the connection");

        // The schema name holds the process id, so two runs at once do not
        // collide. The server reserves the prefix `pg_`, so the name does
        // not start with it.
        let schema = format!("catalog_live_{}", std::process::id());
        client
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
            .expect("the old schema goes");
        client
            .batch_execute(&setup_sql(&schema))
            .expect("the setup runs");

        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let loaded = load(&mut client);
            assert_eq!(loaded.diagnostics, Vec::new(), "the load has no diagnostic");
            check_custom_entities(&loaded.catalog, &schema);
        }));

        client
            .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
            .expect("the schema goes");

        if let Err(payload) = outcome {
            std::panic::resume_unwind(payload);
        }
    }

    #[test]
    fn it_loads_the_temporary_schema_of_the_session() {
        let Some(url) = url() else {
            skip("it_loads_the_temporary_schema_of_the_session");
            return;
        };
        let _guard = lock_server();
        let mut client = Client::connect(&url, NoTls).expect("the server accepts the connection");

        // A temporary table lives as long as the session, so the same client
        // makes it and loads it. The server drops it when the client goes.
        client
            .batch_execute(
                "CREATE TEMP TABLE catalog_live_nocols ();
                 CREATE TEMP TABLE catalog_live_temp (id int4 NOT NULL, label text);",
            )
            .expect("the setup runs");

        let loaded = load(&mut client);
        assert_eq!(loaded.diagnostics, Vec::new(), "the load has no diagnostic");
        let catalog = &loaded.catalog;

        // The catalog holds one temporary schema, with the name that the
        // server gave it. The TOAST schema beside it is not here.
        let temporary = catalog
            .temporary_schema()
            .expect("the session has a temporary schema");
        let schema_name = catalog[temporary].name.as_str().to_owned();
        assert!(schema_name.starts_with("pg_temp_"), "{schema_name}");
        let flagged = catalog
            .schemas()
            .filter(|(_, schema)| schema.is_temporary)
            .count();
        assert_eq!(flagged, 1, "one schema is temporary");
        assert!(
            catalog
                .schemas()
                .all(|(_, schema)| !schema.name.as_str().starts_with("pg_toast")),
            "no TOAST schema is here"
        );

        // `pg_temp` on a search path is the temporary schema.
        let int4 = catalog.builtin(Builtin::Int4);
        let path = SearchPath::new(catalog, ["pg_temp", "public"]);
        let handle = catalog
            .relation_visible(&path, "catalog_live_temp")
            .expect("pg_temp resolves the temporary table");
        let relation = &catalog[handle];
        assert_eq!(relation.schema, temporary);
        assert_eq!(relation.kind, RelationKind::Table);
        assert_eq!(relation.columns.len(), 2);
        assert_eq!(relation.column("id").expect("id is there").ty, int4);

        // The name that the server gave the schema finds the same relation.
        assert_eq!(
            catalog.relation_named(&QualifiedName::new(
                schema_name.as_str(),
                "catalog_live_temp"
            )),
            Some(handle)
        );

        // A table with no column loads, and has a row type with no field.
        let nocols = catalog
            .relation_visible(&path, "catalog_live_nocols")
            .expect("pg_temp resolves the table with no column");
        assert_eq!(catalog[nocols].columns, Vec::new());

        // The path of the session holds the temporary schema too, and so
        // does a path that does not name it.
        for path in [
            SearchPath::from_metadata(catalog),
            SearchPath::new(catalog, ["public"]),
        ] {
            assert_eq!(path.schemas().first(), Some(&temporary));
            assert_eq!(
                catalog.relation_visible(&path, "catalog_live_temp"),
                Some(handle)
            );
        }
    }

    /// The SQL that makes one of every kind of custom entity.
    fn setup_sql(schema: &str) -> String {
        format!(
            "CREATE SCHEMA {schema};
             CREATE TYPE {schema}.mood AS ENUM ('sad', 'ok', 'happy');
             CREATE DOMAIN {schema}.positive AS int4 CHECK (VALUE > 0);
             CREATE TYPE {schema}.point2d AS (x float8, y float8);
             CREATE TABLE {schema}.reading (
                 id int4 NOT NULL,
                 how {schema}.mood,
                 amount {schema}.positive DEFAULT 1,
                 place {schema}.point2d,
                 tags text[]
             );
             CREATE SEQUENCE {schema}.reading_id_seq;
             CREATE FUNCTION {schema}.split(IN a int4, OUT lo int4, OUT hi int4)
                 AS $$ SELECT a - 1, a + 1 $$ LANGUAGE sql;
             CREATE FUNCTION {schema}.total(VARIADIC arr int4[]) RETURNS int4
                 AS $$ SELECT coalesce(sum(x), 0)::int4 FROM unnest(arr) AS x $$
                 LANGUAGE sql IMMUTABLE STRICT;
             CREATE FUNCTION {schema}.add3(int4, int4) RETURNS int4
                 AS $$ SELECT $1 + $2 + 3 $$ LANGUAGE sql IMMUTABLE STRICT;
             CREATE OPERATOR {schema}.### (
                 LEFTARG = int4, RIGHTARG = int4, FUNCTION = {schema}.add3
             );"
        )
    }

    /// Check that every custom entity arrived with the right shape.
    fn check_custom_entities(catalog: &Catalog, schema: &str) {
        let int4 = catalog.builtin(Builtin::Int4);
        let float8 = catalog.builtin(Builtin::Float8);
        let text_array = catalog.builtin(Builtin::TextArray);
        let named = |name: &str| QualifiedName::new(schema, name);

        // The enum keeps its labels, in order.
        let mood = catalog.type_named(&named("mood")).expect("mood is a type");
        assert_eq!(
            catalog[mood].kind,
            TypeKind::Enum {
                labels: vec![Name::new("sad"), Name::new("ok"), Name::new("happy")],
            }
        );

        // The domain keeps its base type and counts its CHECK.
        let positive = catalog
            .type_named(&named("positive"))
            .expect("positive is a type");
        let TypeKind::Domain(domain) = &catalog[positive].kind else {
            panic!("positive is a domain");
        };
        assert_eq!(domain.base, int4);
        assert_eq!(domain.check_constraints, 1);
        assert!(!domain.not_null);

        // The standalone composite has fields and no relation.
        let point2d = catalog
            .type_named(&named("point2d"))
            .expect("point2d is a type");
        let TypeKind::Composite(composite) = &catalog[point2d].kind else {
            panic!("point2d is a composite");
        };
        assert_eq!(composite.relation, None, "a bare composite has no relation");
        assert_eq!(composite.fields.len(), 2);
        assert_eq!(composite.fields[0].name, Name::new("x"));
        assert_eq!(composite.fields[0].ty, float8);
        assert_eq!(composite.fields[1].name, Name::new("y"));

        // The table uses each custom type.
        let reading = catalog
            .relation_named(&named("reading"))
            .expect("reading is a relation");
        let relation = &catalog[reading];
        assert_eq!(relation.kind, RelationKind::Table);
        assert_eq!(relation.columns.len(), 5);
        let column = |name: &str| relation.column(name).expect("the column is there");
        assert_eq!(column("id").ty, int4);
        assert!(column("id").not_null);
        assert_eq!(column("how").ty, mood);
        assert_eq!(column("amount").ty, positive);
        assert!(column("amount").has_default);
        assert_eq!(column("place").ty, point2d);
        assert_eq!(column("tags").ty, text_array);
        let row_type = relation.row_type.expect("a table has a row type");
        let TypeKind::Composite(row_type) = &catalog[row_type].kind else {
            panic!("the row type of the table is a composite");
        };
        assert_eq!(row_type.relation, Some(reading));

        // The sequence is a relation with its three fixed columns.
        let int8 = catalog.builtin(Builtin::Int8);
        let boolean = catalog.builtin(Builtin::Bool);
        let sequence = catalog
            .relation_named(&named("reading_id_seq"))
            .expect("reading_id_seq is a relation");
        let relation = &catalog[sequence];
        assert_eq!(relation.kind, RelationKind::Sequence);
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
        assert!(relation.columns.iter().all(|column| column.not_null));
        assert_eq!(relation.row_type, None, "a sequence has no row type");

        // The function with OUT arguments keeps every argument, and only
        // one of them is an input.
        let split = catalog.functions_named(&named("split"));
        assert_eq!(split.len(), 1, "split has one overload");
        let split = &catalog[split[0]];
        assert_eq!(split.kind, FunctionKind::Function);
        assert_eq!(split.arguments.len(), 3);
        assert_eq!(split.arguments[0].mode, ArgMode::In);
        assert_eq!(split.arguments[0].name, Some(Name::new("a")));
        assert_eq!(split.arguments[1].mode, ArgMode::Out);
        assert_eq!(split.arguments[1].name, Some(Name::new("lo")));
        assert_eq!(split.arguments[2].mode, ArgMode::Out);
        assert_eq!(split.input_arguments().count(), 1);
        assert_eq!(
            catalog.qualified_name_of_type(split.returns).name,
            Name::new("record")
        );

        // The variadic function names the element type of the array.
        let total = catalog.functions_named(&named("total"));
        assert_eq!(total.len(), 1, "total has one overload");
        let total = &catalog[total[0]];
        assert_eq!(total.variadic, Some(int4));
        assert_eq!(total.arguments.len(), 1);
        assert_eq!(total.arguments[0].mode, ArgMode::Variadic);
        assert_eq!(
            catalog.qualified_name_of_type(total.arguments[0].ty).name,
            Name::new("_int4")
        );
        assert_eq!(total.returns, int4);
        assert!(total.strict);

        // The custom operator points at its function.
        let operators = catalog.operators_named(&named("###"));
        assert_eq!(operators.len(), 1, "### has one overload");
        let operator = &catalog[operators[0]];
        assert_eq!(operator.left, Some(int4));
        assert_eq!(operator.right, int4);
        assert_eq!(operator.result, int4);
        assert_eq!(
            catalog.qualified_name_of_function(operator.function),
            named("add3")
        );

        // The schema itself is there, and belongs to no extension.
        let handle = catalog.schema_named(schema).expect("the schema is there");
        assert_eq!(catalog[handle].extension, None);
    }
}

#[cfg(feature = "tokio-postgres")]
mod async_tests {
    use super::{lock_server, skip, url};
    use pg_catalog::{Builtin, Name};

    // The guard stays across each `await`. This is safe here, because the
    // test is the only task on its runtime, and the lock keeps two test
    // threads off one server. An async lock would need another dependency.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn it_loads_through_the_async_executor() {
        let Some(url) = url() else {
            skip("it_loads_through_the_async_executor");
            return;
        };
        let _guard = lock_server();
        let (mut client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
            .await
            .expect("the server accepts the connection");
        tokio::spawn(async move {
            let _ = connection.await;
        });

        let loaded = pg_catalog::load_async(&mut client)
            .await
            .expect("the load succeeds");
        assert_eq!(loaded.diagnostics, Vec::new(), "the load has no diagnostic");

        let catalog = &loaded.catalog;
        let int4 = catalog.builtin(Builtin::Int4);
        assert_eq!(catalog[int4].name, Name::new("int4"));
        assert!(catalog.types().len() > 100, "a server has many types");
    }
}
