//! Diagnostic, no server needed: which statements of the psql submissions of
//! the frozen corpus does `pgtk-sql` still reject? Run with
//! `cargo test --test corpus_parse -- --ignored --nocapture`.

mod support;

use support::corpus::{FrozenSplit, ItemKind, schedule};
use support::replay::{parse, statements};

#[test]
#[ignore = "diagnostic listing, not a gate"]
fn list_parse_failures() {
    let split = FrozenSplit::load();
    let mut total = 0;
    for name in schedule() {
        if !split.file_names().any(|known| known == name) {
            continue;
        }
        let file = split.read(&name);
        for item in &file.items {
            if item.kind != ItemKind::Statement {
                continue;
            }
            for sql in statements(file.text_of(item)) {
                if parse(&sql).is_some() {
                    continue;
                }
                total += 1;
                let shown = sql.replace('\n', " ").chars().take(110).collect::<String>();
                println!("PARSE {name}: {shown}");
            }
        }
    }
    println!("total {total}");
}
