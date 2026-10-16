//! Diagnostic, no server needed: which frozen corpus statements does
//! `pgtk-sql` still reject after `pgtk-psql` has rendered them? Run with
//! `cargo test --test corpus_parse -- --ignored --nocapture`.

mod support;

use support::corpus::{FrozenSplit, ItemKind, schedule};
use support::replay::{parse, render};

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
            let raw = file.text_of(item);
            let status = match render(raw) {
                None => "RENDER",
                Some(sql) if parse(&sql).is_none() => "PARSE ",
                Some(_) => continue,
            };
            total += 1;
            let shown = raw.replace('\n', " ").chars().take(110).collect::<String>();
            println!("{status} {name}: {shown}");
        }
    }
    println!("total {total}");
}
