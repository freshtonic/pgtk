//! The regression corpus: PostgreSQL's `src/test/regress/sql/*.sql` from the
//! `vendor/postgres` submodule, split into statements by the byte ranges
//! `pgtk-sql` froze when it captured its own differential baseline, copied to
//! `baselines/postgresql-17.9-statements.json`.
//!
//! The files are psql documents, and 97 of them use meta-commands such as
//! `\set` and `\d` that neither `pgtk-sql` nor `pgtk-psql` model, so the strict
//! document parser rejects them whole. The frozen ranges came from a lenient
//! splitter and mark each item as a statement (`S`) or a non-statement (`E`).

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Command;

/// This crate's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The vendored PostgreSQL repository, the `vendor/postgres` submodule at the
/// repository root. Its checkout can be a later release than the corpus, so
/// the tests read the corpus from its object store, not from its checkout.
pub fn postgres_dir() -> PathBuf {
    root().join("../../vendor/postgres")
}

/// The frozen statement ranges, shared with pgtk-sql.
fn frozen_split_path() -> PathBuf {
    root().join("baselines/postgresql-17.9-statements.json")
}

/// Runs git in the vendored PostgreSQL repository and returns its output, or
/// why it failed.
fn git(arguments: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(postgres_dir())
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(output.stdout)
}

/// Panics when the object store of the vendored PostgreSQL does not hold
/// `commit`.
fn require_commit(commit: &str) {
    if let Err(error) = git(&["cat-file", "-e", &format!("{commit}^{{commit}}")]) {
        panic!(
            "vendor/postgres does not hold the commit {commit} that the statement ranges \
             were frozen from ({error}); run `git submodule update --init` with full history"
        );
    }
}

/// The text of the object `spec` (`<commit>:<path>` or a blob ID) in the
/// vendored PostgreSQL.
fn object_text(spec: &str) -> String {
    let bytes = git(&["cat-file", "-p", spec])
        .unwrap_or_else(|error| panic!("cannot read {spec} from vendor/postgres: {error}"));
    String::from_utf8(bytes).unwrap_or_else(|_| panic!("{spec} is not UTF-8"))
}

/// A file of the PostgreSQL commit that the statement ranges were frozen
/// from, such as `src/test/regress/parallel_schedule`.
pub fn frozen_file(path: &str) -> String {
    let text = std::fs::read_to_string(frozen_split_path()).expect("frozen split");
    let document: serde_json::Value = serde_json::from_str(&text).expect("frozen split JSON");
    let gitlink = document["postgres"]["gitlink"]
        .as_str()
        .expect("frozen split records the PostgreSQL commit");
    require_commit(gitlink);
    object_text(&format!("{gitlink}:{path}"))
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
    files: BTreeMap<String, FrozenFile>,
    gitlink: String,
}

/// The frozen split of one file.
struct FrozenFile {
    /// The Git blob ID of the file in the frozen commit.
    blob: String,
    /// The length of the file in bytes.
    bytes: usize,
    /// The frozen items in source order.
    items: Vec<Item>,
}

impl FrozenSplit {
    /// Read the frozen statement ranges, after checking that the vendored
    /// PostgreSQL holds the commit they were frozen from.
    pub fn load() -> Self {
        let path = frozen_split_path();
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let document: serde_json::Value = serde_json::from_str(&text).expect("frozen split JSON");
        assert_eq!(
            document["encoding"].as_str(),
            Some("comma-separated-byte-ranges-v1"),
            "frozen split encoding changed"
        );
        let gitlink = document["postgres"]["gitlink"]
            .as_str()
            .expect("frozen split records the PostgreSQL commit");
        require_commit(gitlink);
        let files = document["files"]
            .as_array()
            .expect("frozen split files")
            .iter()
            .map(|file| {
                let name = file["file"].as_str().expect("file name").to_owned();
                let blob = file["source_git_blob"]
                    .as_str()
                    .expect("source_git_blob")
                    .to_owned();
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
                (name, FrozenFile { blob, bytes, items })
            })
            .collect();
        Self {
            files,
            gitlink: gitlink.to_owned(),
        }
    }

    /// The PostgreSQL commit the ranges were frozen from.
    pub fn gitlink(&self) -> &str {
        &self.gitlink
    }

    /// Every file name in the split.
    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// Read one corpus file, by its frozen blob ID, and attach its frozen
    /// split.
    pub fn read(&self, name: &str) -> CorpusFile {
        let file = self
            .files
            .get(name)
            .unwrap_or_else(|| panic!("{name} has no frozen split"));
        let text = object_text(&file.blob);
        assert_eq!(
            text.len(),
            file.bytes,
            "{name}: the blob is not the file the split was frozen from"
        );
        CorpusFile {
            name: name.to_owned(),
            text,
            items: file.items.clone(),
        }
    }
}

/// The order pg_regress runs the files in, from `parallel_schedule`.
///
/// Later files depend on tables that earlier files make, so the replay must
/// follow this order. Files in one parallel group run in the listed order,
/// which is one valid serialisation of the group.
pub fn schedule() -> Vec<String> {
    frozen_file("src/test/regress/parallel_schedule")
        .lines()
        .filter_map(|line| line.strip_prefix("test:"))
        .flat_map(|names| names.split_whitespace().map(|name| format!("{name}.sql")))
        .collect()
}
