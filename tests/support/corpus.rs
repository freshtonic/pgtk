//! The regression corpus: PostgreSQL's `src/test/regress/sql/*.sql`, split
//! into statements by the byte ranges `pg-sql` froze when it captured its own
//! differential baseline.
//!
//! The files are psql documents, and 97 of them use meta-commands such as
//! `\set` and `\d` that neither `pg-sql` nor `pg-psql` model, so the strict
//! document parser rejects them whole. The frozen ranges came from a lenient
//! splitter and mark each item as a statement (`S`) or a non-statement (`E`).

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// The `pg-sql` checkout that vendors PostgreSQL and holds the frozen ranges.
pub fn pg_sql_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../pg-sql")
}

/// The directory of the regression SQL files.
pub fn corpus_dir() -> PathBuf {
    pg_sql_root().join("vendor/postgres/src/test/regress/sql")
}

/// What one frozen item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// A SQL statement the lenient splitter parsed.
    Statement,
    /// Anything else: a psql meta-command, or text the splitter could not parse.
    Other,
}

/// One item of one corpus file.
#[derive(Clone, Debug)]
pub struct Item {
    /// The byte range in the file.
    pub range: Range<usize>,
    /// What the frozen split says the item is.
    pub kind: ItemKind,
}

/// One corpus file with its frozen split.
#[derive(Debug)]
pub struct CorpusFile {
    /// The file name, such as `int4.sql`.
    pub name: String,
    /// The whole text of the file.
    pub text: String,
    /// The frozen items in source order.
    pub items: Vec<Item>,
}

impl CorpusFile {
    /// The text of one item.
    pub fn text_of(&self, item: &Item) -> &str {
        &self.text[item.range.clone()]
    }
}

/// The frozen split of every file, keyed by file name.
pub struct FrozenSplit {
    files: BTreeMap<String, (usize, Vec<Item>)>,
}

impl FrozenSplit {
    /// Read `pg-sql`'s frozen statement ranges.
    pub fn load() -> Self {
        let path = pg_sql_root().join("baselines/postgresql-17.9-statements.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let document: serde_json::Value = serde_json::from_str(&text).expect("frozen split JSON");
        assert_eq!(
            document["encoding"].as_str(),
            Some("comma-separated-byte-ranges-v1"),
            "frozen split encoding changed"
        );
        let files = document["files"]
            .as_array()
            .expect("frozen split files")
            .iter()
            .map(|file| {
                let name = file["file"].as_str().expect("file name").to_owned();
                let bytes = file["source_bytes"].as_u64().expect("source_bytes") as usize;
                let kinds = file["legacy_item_kinds"]
                    .as_str()
                    .expect("legacy_item_kinds");
                let ranges = file["byte_ranges"].as_str().expect("byte_ranges");
                let items = if ranges.is_empty() {
                    Vec::new()
                } else {
                    ranges
                        .split(',')
                        .zip(kinds.chars())
                        .map(|(range, kind)| {
                            let (start, end) = range.split_once(':').expect("range `start:end`");
                            let range = start.parse().expect("range start")
                                ..end.parse().expect("range end");
                            let kind = match kind {
                                'S' => ItemKind::Statement,
                                'E' => ItemKind::Other,
                                other => panic!("unknown item kind {other:?} in {name}"),
                            };
                            Item { range, kind }
                        })
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    items.len(),
                    kinds.len(),
                    "{name}: range and kind counts differ"
                );
                (name, (bytes, items))
            })
            .collect();
        Self { files }
    }

    /// Every file name in the split.
    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// Read one corpus file and attach its frozen split.
    ///
    /// Panics when the file on disk is not the file the split was made from.
    pub fn read(&self, name: &str) -> CorpusFile {
        let (bytes, items) = self
            .files
            .get(name)
            .unwrap_or_else(|| panic!("{name} has no frozen split"));
        let path = corpus_dir().join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        assert_eq!(
            text.len(),
            *bytes,
            "{name}: the file on disk is not the file the split was frozen from"
        );
        CorpusFile {
            name: name.to_owned(),
            text,
            items: items.clone(),
        }
    }
}

/// The order pg_regress runs the files in, from `parallel_schedule`.
///
/// Later files depend on tables that earlier files make, so the replay must
/// follow this order. Files in one parallel group run in the listed order,
/// which is one valid serialisation of the group.
pub fn schedule() -> Vec<String> {
    let path = pg_sql_root().join("vendor/postgres/src/test/regress/parallel_schedule");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    text.lines()
        .filter_map(|line| line.strip_prefix("test:"))
        .flat_map(|names| names.split_whitespace().map(|name| format!("{name}.sql")))
        .collect()
}
