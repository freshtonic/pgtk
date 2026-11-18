//! Deterministically regenerates stress-test SQL fixtures under
//! `pgtk-sql/fixtures/stress/`. Run with `cargo run --bin gen-stress -p pgtk-sql`.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("fixtures/stress");
    fs::create_dir_all(&out).expect("create stress dir");

    for n in [100usize, 1_000, 10_000] {
        write(&out, &format!("insert_values_{n}.sql"), &insert_values(n));
    }
    for n in [10usize, 100, 1_000] {
        write(&out, &format!("bool_chain_{n}.sql"), &bool_chain(n));
    }
    for n in [100usize, 1_000, 10_000] {
        write(&out, &format!("select_list_{n}.sql"), &select_list(n));
    }
    for n in [5usize, 10, 15] {
        write(
            &out,
            &format!("nested_subquery_{n}.sql"),
            &nested_subquery(n),
        );
    }
    for n in [100usize, 1_000, 10_000] {
        write(&out, &format!("in_list_{n}.sql"), &in_list(n));
    }
    write(&out, "lexical_mix_1000.sql", &lexical_mix(1_000));

    // The document-level sweeps of the flame harness. Each moves one axis of
    // `document::parse_sql` and holds the other two still; the sizes must
    // match `DOCUMENT_AXES` in `benches/flame.rs`.
    for n in [500usize, 1_000, 2_000, 4_000] {
        write(
            &out,
            &format!("document_islands_{n}.sql"),
            &document_islands(n),
        );
    }
    for n in [250usize, 500, 1_000, 2_000] {
        write(
            &out,
            &format!("document_island_size_{n}.sql"),
            &document_island_size(n),
        );
    }
    for n in [8usize, 16, 32, 64, 128] {
        write(
            &out,
            &format!("document_boundary_depth_{n}.sql"),
            &document_boundary_depth(n),
        );
    }

    println!("wrote stress fixtures to {}", out.display());
}

fn write(dir: &Path, name: &str, contents: &str) {
    let path = dir.join(name);
    fs::write(&path, contents).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn insert_values(n: usize) -> String {
    let mut s = String::from("INSERT INTO t (a, b, c, d) VALUES\n");
    for i in 0..n {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&format!("  ({i}, 'x{i}', {i}.5, true)"));
    }
    s.push_str(";\n");
    s
}

fn bool_chain(n: usize) -> String {
    let mut s = String::from("SELECT 1 WHERE ");
    for i in 0..n {
        if i > 0 {
            s.push_str(" AND ");
        }
        s.push('a');
    }
    s.push_str(";\n");
    s
}

fn select_list(n: usize) -> String {
    let mut s = String::from("SELECT ");
    for i in 0..n {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("c{i}"));
    }
    s.push_str(" FROM t;\n");
    s
}

fn nested_subquery(n: usize) -> String {
    let mut s = String::from("SELECT * FROM ");
    for _ in 0..n {
        s.push_str("(SELECT * FROM ");
    }
    s.push('t');
    for i in 0..n {
        s.push_str(&format!(") s{i}"));
    }
    s.push_str(";\n");
    s
}

fn in_list(n: usize) -> String {
    let mut s = String::from("SELECT * FROM t WHERE x IN (");
    for i in 0..n {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("{i}"));
    }
    s.push_str(");\n");
    s
}

/// A wide statement whose target entries rotate through PostgreSQL's major
/// literal and identifier spellings. Comments are deliberately interleaved so
/// the lexer also exercises trivia ownership without turning this into a
/// document-level benchmark.
fn lexical_mix(n: usize) -> String {
    let mut s = String::from("SELECT\n");
    for i in 0..n {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&format!("  /* lexical item {i} */ "));
        match i % 6 {
            0 => s.push_str(&format!("\"quoted identifier {i}\"")),
            1 => s.push_str(&format!("'single '' quoted {i}'")),
            2 => s.push_str(&format!("E'escaped \\\\n \\\\x41 {i}'")),
            3 => s.push_str(&format!("$tag_{i}$dollar quoted body {i}$tag_{i}$")),
            4 => s.push_str(&format!("123456789.{i}e-3")),
            _ => s.push_str(&format!("(1 + {i})::numeric")),
        }
    }
    s.push_str(";\n");
    s
}

/// Island count: `n` short statements of eight families, with a line
/// comment before every eighth. Each statement has one candidate boundary,
/// its own `;`, and about the same length whatever `n` is.
fn document_islands(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        if i % 8 == 0 {
            s.push_str(&format!("-- batch {}\n", i / 8));
        }
        let statement = match i % 8 {
            0 => format!("SELECT id, name FROM accounts WHERE id = {i};"),
            1 => format!("INSERT INTO events (id, kind) VALUES ({i}, 'k{i}');"),
            2 => format!("UPDATE accounts SET balance = balance + {i} WHERE id = {i};"),
            3 => format!("DELETE FROM sessions WHERE expires < {i};"),
            4 => format!("CREATE TABLE t{i} (id int PRIMARY KEY, v text);"),
            5 => format!("GRANT SELECT ON t{i} TO reader;"),
            6 => format!("SET search_path TO s{i}, public;"),
            _ => "COMMIT;".to_owned(),
        };
        s.push_str(&statement);
        s.push('\n');
    }
    s
}

/// Island size: four statements of different shapes, each with `n` list
/// items and one candidate boundary. The island count stays four.
fn document_island_size(n: usize) -> String {
    let list = |item: &dyn Fn(usize) -> String, separator: &str| {
        (0..n).map(item).collect::<Vec<_>>().join(separator)
    };
    let mut s = String::from("INSERT INTO t (a, b) VALUES\n  ");
    s.push_str(&list(&|i| format!("({i}, 'v{i}')"), ",\n  "));
    s.push_str(";\n\nSELECT ");
    s.push_str(&list(&|i| format!("a + {i} AS x{i}"), ", "));
    s.push_str(" FROM t;\n\nSELECT * FROM t WHERE k IN (");
    s.push_str(&list(&|i| i.to_string(), ", "));
    s.push_str(");\n\nSELECT CASE x ");
    s.push_str(&list(&|i| format!("WHEN {i} THEN 'w{i}'"), " "));
    s.push_str(" END FROM t;\n");
    s
}

/// Boundary-candidate depth: four statements, each holding `depth`
/// semicolons that are not statement boundaries, so framing meets `depth`
/// candidate boundaries in each before the one that ends it. Two are
/// `BEGIN ATOMIC` function bodies and two are multi-action rules, the two
/// places where `gram.y` puts a `;` inside a statement. Each statement
/// spends the same budget of about 2,400 tokens, shared among its inner
/// statements as column lists, so the island size and count stay the same
/// while the depth moves.
fn document_boundary_depth(depth: usize) -> String {
    const TOKENS: usize = 2_400;
    // The column lists of `parts` inner statements that each cost `overhead`
    // tokens besides their columns. A column costs two tokens with its comma.
    let column_lists = |parts: usize, overhead: usize| {
        let items = (TOKENS - parts * overhead) / 2;
        (0..parts)
            .map(|k| {
                (k * items / parts..(k + 1) * items / parts)
                    .map(|i| format!("c{i}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect::<Vec<_>>()
    };
    let mut s = String::new();
    for island in 0..4 {
        if island % 2 == 0 {
            // `routine_body_stmt_list` ends every inner statement with `;`.
            // `SELECT`, `FROM`, `t` and `;`, less the comma it does not have.
            s.push_str(&format!(
                "CREATE FUNCTION f{island}() RETURNS int LANGUAGE sql\nBEGIN ATOMIC\n"
            ));
            for columns in column_lists(depth, 3) {
                s.push_str(&format!("  SELECT {columns} FROM t;\n"));
            }
            s.push_str("END;\n\n");
        } else {
            // `RuleActionMulti` puts a `;` between its actions.
            // `INSERT INTO log VALUES (`, `)` and `;`, less the missing comma.
            s.push_str(&format!(
                "CREATE RULE r{island} AS ON INSERT TO t DO ALSO (\n  "
            ));
            let actions = column_lists(depth + 1, 6)
                .into_iter()
                .map(|columns| format!("INSERT INTO log VALUES ({columns})"))
                .collect::<Vec<_>>();
            s.push_str(&actions.join(";\n  "));
            s.push_str("\n);\n\n");
        }
    }
    s
}
