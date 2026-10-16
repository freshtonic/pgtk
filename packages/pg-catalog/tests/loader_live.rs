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
        ArgMode, Builtin, CastMethod, Catalog, ConstraintKind, FiresWhen, FunctionKind, Identity,
        Loaded, Name, ParameterKind, PrimaryKey, QualifiedName, RelationKind, RuleEvent,
        SearchPath, TriggerTiming, TypeKind,
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

        // `int4` sorts and hashes. `json` does neither.
        let btree = catalog
            .btree_ordering(int4)
            .expect("int4 has a default btree operator class");
        assert_eq!(catalog[btree.lt].name, Name::new("<"));
        assert_eq!(catalog[btree.gt].name, Name::new(">"));
        assert_eq!(catalog.hash_equality(int4), Some(btree.eq));
        let json = catalog.builtin(Builtin::Json);
        assert_eq!(catalog.btree_ordering(json), None);
        assert_eq!(catalog.hash_equality(json), None);

        // The loader reads the type of each default of each stock function.
        for (_, function) in catalog.functions() {
            assert_eq!(
                function.default_types.len(),
                usize::from(function.default_count)
            );
            assert!(
                function.default_types.iter().all(Option::is_some),
                "a default of {} has no type",
                function.name
            );
        }

        // The database has an encoding, and the three collations that each
        // server has are there for it.
        let encoding = &catalog.metadata().encoding;
        assert!(!encoding.name.is_empty());
        for name in ["default", "C", "POSIX"] {
            let handle = catalog
                .collation_named(&QualifiedName::new("pg_catalog", name))
                .unwrap_or_else(|| panic!("{name} is a collation"));
            assert!(encoding.can_use(&catalog[handle]));
        }
        for (_, ty) in catalog.types() {
            assert_eq!(ty.is_collatable, ty.collation.is_some(), "{}", ty.name);
        }

        // `SHOW timezone` names its column `TimeZone`.
        let timezone = catalog.parameter("timezone").expect("TimeZone is there");
        assert_eq!(timezone.name, "TimeZone");
        assert_eq!(timezone.kind, ParameterKind::String);
        assert_eq!(
            catalog.parameter("WORK_MEM").map(|p| p.kind),
            Some(ParameterKind::Integer)
        );
        assert_eq!(catalog.parameter("no_such_parameter"), None);

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
                 CREATE TEMP TABLE catalog_live_temp (id int4 NOT NULL, label text);
                 CREATE TEMP TABLE catalog_live_keyed (id int PRIMARY KEY, name text);
                 CREATE TEMP TABLE catalog_live_pair (
                     a int, b int, c text, PRIMARY KEY (b, a)
                 );
                 CREATE TEMP TABLE catalog_live_identity (
                     always int GENERATED ALWAYS AS IDENTITY,
                     by_default int GENERATED BY DEFAULT AS IDENTITY,
                     plain int,
                     doubled int GENERATED ALWAYS AS (plain * 2) STORED
                 );
                 CREATE TEMP TABLE catalog_live_deferred (
                     id int PRIMARY KEY DEFERRABLE, name text
                 );",
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

        // A primary key arrives in key order, with its deferrable flag. A
        // table with no key has none.
        let key_of = |name: &str| {
            let handle = catalog
                .relation_visible(&path, name)
                .expect("pg_temp resolves the table");
            catalog[handle].primary_key.clone()
        };
        assert_eq!(
            key_of("catalog_live_keyed"),
            Some(PrimaryKey {
                columns: vec![1],
                deferrable: false,
            })
        );
        assert_eq!(
            key_of("catalog_live_pair"),
            Some(PrimaryKey {
                columns: vec![2, 1],
                deferrable: false,
            })
        );
        let deferred = key_of("catalog_live_deferred").expect("the table has a key");
        assert!(deferred.deferrable);
        assert_eq!(deferred.grouping_columns(), None);
        assert_eq!(key_of("catalog_live_temp"), None);

        // An identity column keeps its kind. A generated column is not an
        // identity column.
        let identity = catalog
            .relation_visible(&path, "catalog_live_identity")
            .expect("pg_temp resolves the table");
        let column = |name: &str| catalog[identity].column(name).expect("the column is there");
        assert_eq!(column("always").identity, Some(Identity::Always));
        assert_eq!(column("by_default").identity, Some(Identity::ByDefault));
        assert_eq!(column("plain").identity, None);
        assert_eq!(column("doubled").identity, None);
        assert!(column("doubled").generated);
        assert!(!column("always").generated);

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
             CREATE TABLE {schema}.points OF {schema}.point2d;
             CREATE VIEW {schema}.reading_ids AS SELECT id FROM {schema}.reading;
             CREATE MATERIALIZED VIEW {schema}.reading_count AS
                 SELECT count(*) AS n FROM {schema}.reading;
             CREATE VIEW {schema}.doubled AS SELECT id * 2 AS twice FROM {schema}.reading;
             CREATE RULE doubled_ins AS ON INSERT TO {schema}.doubled DO INSTEAD NOTHING;
             CREATE RULE doubled_del AS ON DELETE TO {schema}.doubled
                 WHERE old.twice > 10 DO INSTEAD NOTHING;
             CREATE FUNCTION {schema}.noop() RETURNS trigger
                 AS $$ BEGIN RETURN NEW; END $$ LANGUAGE plpgsql;
             CREATE TRIGGER doubled_upd INSTEAD OF UPDATE ON {schema}.doubled
                 FOR EACH ROW EXECUTE FUNCTION {schema}.noop();
             CREATE TRIGGER reading_audit AFTER INSERT OR DELETE ON {schema}.reading
                 FOR EACH STATEMENT EXECUTE FUNCTION {schema}.noop();
             ALTER TABLE {schema}.reading ADD PRIMARY KEY (id);
             CREATE TABLE {schema}.reading_note (
                 reading_id int4 REFERENCES {schema}.reading (id)
             );
             CREATE RULE note_off AS ON UPDATE TO {schema}.reading_note DO INSTEAD NOTHING;
             ALTER TABLE {schema}.reading_note DISABLE RULE note_off;
             CREATE COLLATION {schema}.shouty (LOCALE = 'C');
             CREATE TABLE {schema}.worded (
                 plain text,
                 c_text text COLLATE \"C\",
                 posix_text text COLLATE \"POSIX\",
                 shouty_text text COLLATE {schema}.shouty,
                 n int4
             );
             CREATE DOMAIN {schema}.c_word AS text COLLATE \"C\";
             CREATE TABLE {schema}.booked (
                 id int4 CONSTRAINT booked_pk PRIMARY KEY,
                 room int4 NOT NULL CONSTRAINT booked_room_key UNIQUE DEFERRABLE,
                 seats int4 CONSTRAINT booked_seats_check CHECK (seats > 0),
                 during int4range,
                 CONSTRAINT booked_no_overlap EXCLUDE USING gist (during WITH &&),
                 CONSTRAINT booked_always CHECK (true)
             );
             CREATE VIEW {schema}.booked_view AS SELECT id, room FROM {schema}.booked;
             CREATE RULE booked_view_ins AS ON INSERT TO {schema}.booked_view
                 DO INSTEAD INSERT INTO {schema}.booked (id, room)
                 VALUES (new.id, new.room) RETURNING id, room;
             CREATE OPERATOR CLASS {schema}.int4_backwards_ops FOR TYPE int4 USING btree AS
                 OPERATOR 1 >, OPERATOR 2 >=, OPERATOR 3 =, OPERATOR 4 <=, OPERATOR 5 <,
                 FUNCTION 1 btint4cmp(int4, int4);
             CREATE TABLE {schema}.p1 (a int4);
             CREATE TABLE {schema}.p2 (b text);
             CREATE TABLE {schema}.c1 (c int4) INHERITS ({schema}.p2, {schema}.p1);
             CREATE TABLE {schema}.measured (at date) PARTITION BY RANGE (at);
             CREATE TABLE {schema}.measured_2026 PARTITION OF {schema}.measured
                 FOR VALUES FROM ('2026-01-01') TO ('2027-01-01');
             CREATE INDEX ON {schema}.measured (at);
             CREATE FUNCTION {schema}.dfunc(
                 a int4,
                 b anyelement = 'World'::text,
                 c anyelement = 'x',
                 d timestamptz = now(),
                 e int4[] = ARRAY[1, 2],
                 f int4 = 1 + 1,
                 g text = NULL
             ) RETURNS int4 AS $$ SELECT 1 $$ LANGUAGE sql;
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

        // A table names its direct parents, in the order of `INHERITS`. A
        // partition names its partitioned table. The partition of the index
        // is in `pg_inherits` too, and gives no note.
        let relation = |name: &str| {
            catalog
                .relation_named(&named(name))
                .expect("the relation is there")
        };
        assert_eq!(
            catalog[relation("c1")].parents,
            vec![relation("p2"), relation("p1")]
        );
        assert_eq!(catalog[relation("p1")].parents, Vec::new());
        assert_eq!(
            catalog[relation("measured_2026")].parents,
            vec![relation("measured")]
        );

        // A view and a materialized view have the text of their query. A
        // table has none.
        let text_of = |name: &str| catalog[relation(name)].view_definition.clone();
        let ids = text_of("reading_ids").expect("a view has its text");
        assert!(ids.contains("SELECT") && ids.contains("reading"), "{ids}");
        assert!(
            text_of("reading_count")
                .expect("the text is there")
                .contains("count")
        );
        assert_eq!(text_of("reading"), None);

        // The rules are in name order, without `_RETURN`.
        let doubled = &catalog[relation("doubled")];
        let rules: Vec<_> = doubled
            .rules
            .iter()
            .map(|rule| {
                (
                    rule.name.as_str(),
                    rule.event,
                    rule.instead,
                    rule.unconditional,
                    rule.fires,
                )
            })
            .collect();
        assert_eq!(
            rules,
            vec![
                (
                    "doubled_del",
                    RuleEvent::Delete,
                    true,
                    false,
                    FiresWhen::Origin
                ),
                (
                    "doubled_ins",
                    RuleEvent::Insert,
                    true,
                    true,
                    FiresWhen::Origin
                ),
            ]
        );
        let replaces = |event| doubled.rules.iter().any(|rule| rule.replaces(event));
        assert!(replaces(RuleEvent::Insert));
        assert!(!replaces(RuleEvent::Delete), "the rule has a condition");
        assert_eq!(catalog[relation("reading_ids")].rules, Vec::new());

        // A disabled rule is there, and it does not replace a statement. The
        // server permits `DISABLE RULE` only on a table.
        let note_rules = &catalog[relation("reading_note")].rules;
        assert_eq!(note_rules.len(), 1);
        assert_eq!(note_rules[0].fires, FiresWhen::Disabled);
        assert!(note_rules[0].instead && note_rules[0].unconditional);
        assert!(
            !note_rules[0].replaces(RuleEvent::Update),
            "the rule is disabled"
        );

        // An INSTEAD OF trigger on the view, and a statement trigger on the
        // table. The foreign key makes internal triggers on `reading`, and
        // those are not here.
        assert_eq!(doubled.triggers.len(), 1);
        let trigger = &doubled.triggers[0];
        assert_eq!(trigger.name, Name::new("doubled_upd"));
        assert_eq!(trigger.timing, TriggerTiming::InsteadOf);
        assert!(trigger.update && trigger.row_level && trigger.is_instead_of_row());
        assert!(!trigger.insert && !trigger.delete && !trigger.truncate);

        let reading_triggers = &catalog[relation("reading")].triggers;
        assert_eq!(reading_triggers.len(), 1, "no internal trigger is here");
        let audit = &reading_triggers[0];
        assert_eq!(audit.name, Name::new("reading_audit"));
        assert_eq!(audit.timing, TriggerTiming::After);
        assert!(audit.insert && audit.delete && !audit.update && !audit.row_level);
        assert_eq!(audit.fires, FiresWhen::Origin);

        // Each column has its collation. A column with no COLLATE clause has
        // the `default` collation of `text`. An `int4` column has none.
        let collation = |schema: &str, name: &str| {
            catalog
                .collation_named(&QualifiedName::new(schema, name))
                .expect("the collation is there")
        };
        let default = collation("pg_catalog", "default");
        let shouty = collation(schema, "shouty");
        let worded = &catalog[relation("worded")];
        let collation_of = |name: &str| worded.column(name).expect("the column is there").collation;
        assert_eq!(collation_of("plain"), Some(default));
        assert_eq!(collation_of("c_text"), Some(collation("pg_catalog", "C")));
        assert_eq!(
            collation_of("posix_text"),
            Some(collation("pg_catalog", "POSIX"))
        );
        assert_eq!(collation_of("shouty_text"), Some(shouty));
        assert_eq!(collation_of("n"), None);
        assert_eq!(
            catalog[shouty].schema,
            catalog.schema_named(schema).unwrap()
        );
        assert_eq!(catalog[shouty].extension, None);

        // A domain can have a collation that is not that of its base type.
        let c_word = catalog
            .type_named(&named("c_word"))
            .expect("c_word is a type");
        assert_eq!(
            catalog[c_word].collation,
            Some(collation("pg_catalog", "C"))
        );
        assert_eq!(
            catalog[catalog.builtin(Builtin::Text)].collation,
            Some(default)
        );
        assert_eq!(catalog[int4].collation, None);

        // A relation has its constraints by name. NOT NULL is not a
        // constraint on PostgreSQL 14 to 17.
        let booked = &catalog[relation("booked")];
        let kinds: Vec<_> = booked
            .constraints
            .iter()
            .map(|c| (c.name.as_str(), c.kind, c.deferrable, c.columns.clone()))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("booked_always", ConstraintKind::Check, false, vec![]),
                (
                    "booked_no_overlap",
                    ConstraintKind::Exclusion,
                    false,
                    vec![4]
                ),
                ("booked_pk", ConstraintKind::PrimaryKey, false, vec![1]),
                ("booked_room_key", ConstraintKind::Unique, true, vec![2]),
                ("booked_seats_check", ConstraintKind::Check, false, vec![3]),
            ]
        );
        assert!(booked.constraint("booked_pk").is_some());
        assert!(booked.constraint("no_such_constraint").is_none());
        // The foreign key of `reading_note` is a constraint of that table.
        assert!(
            catalog[relation("reading_note")]
                .constraints
                .iter()
                .any(|c| c.kind == ConstraintKind::ForeignKey)
        );

        // The definition of a rule has the RETURNING list of its action.
        let rules = &catalog[relation("booked_view")].rules;
        assert_eq!(rules.len(), 1);
        let definition = &rules[0].definition;
        assert!(
            definition.starts_with("CREATE RULE booked_view_ins"),
            "{definition}"
        );
        assert!(definition.contains("RETURNING"), "{definition}");
        assert!(rules[0].replaces(RuleEvent::Insert));

        // An operator class answers its name and its access method.
        let path = SearchPath::new(catalog, [schema, "public"]);
        let backwards = catalog
            .operator_class_visible(&path, "int4_backwards_ops", "btree")
            .expect("the new class is there");
        assert_eq!(catalog[backwards].input_type, int4);
        assert!(!catalog[backwards].is_default);
        assert_eq!(catalog[backwards].extension, None);
        assert_eq!(
            catalog.operator_class_visible(&path, "int4_backwards_ops", "hash"),
            None
        );
        let stock = catalog
            .operator_class_visible(&path, "int4_ops", "btree")
            .expect("int4_ops is there");
        assert!(catalog[stock].is_default);
        // The default class of `int4` is still the stock one.
        assert_eq!(
            catalog[catalog.btree_ordering(int4).unwrap().lt].name,
            Name::new("<")
        );

        // A typed table names the type that it is `OF`. Its row type is
        // another type, with the same fields.
        let points = &catalog[relation("points")];
        assert_eq!(points.of_type, Some(point2d));
        assert_ne!(points.row_type, Some(point2d));
        assert_eq!(points.columns.len(), 2);
        assert_eq!(catalog[relation("reading")].of_type, None);

        // A function keeps the type of each default expression. An untyped
        // literal for a polymorphic argument stays `unknown`.
        let dfunc = catalog.functions_named(&named("dfunc"));
        assert_eq!(dfunc.len(), 1, "dfunc has one overload");
        let dfunc = &catalog[dfunc[0]];
        let builtin = |builtin| Some(catalog.builtin(builtin));
        assert_eq!(dfunc.default_count, 6);
        assert_eq!(
            dfunc.default_types,
            vec![
                builtin(Builtin::Text),
                builtin(Builtin::Unknown),
                builtin(Builtin::Timestamptz),
                builtin(Builtin::Int4Array),
                builtin(Builtin::Int4),
                builtin(Builtin::Text),
            ]
        );
        assert_eq!(dfunc.default_type_of(0), None, "a has no default");
        assert_eq!(dfunc.default_type_of(1), builtin(Builtin::Text));
        assert_eq!(dfunc.default_type_of(2), builtin(Builtin::Unknown));

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
