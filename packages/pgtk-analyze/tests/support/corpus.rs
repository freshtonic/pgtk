//! The regression corpus: PostgreSQL's `src/test/regress/sql/*.sql` from the
//! `vendor/postgres` submodule, as psql sends it to the server.
//!
//! The files are psql documents. The harness reads the psql submissions that
//! `pgtk-sql` freezes for its own differential test
//! (`packages/pgtk-sql/baselines/postgresql-17.9-submissions.json`, issue
//! #10), through the same reader (`pgtk-sql/tests/support/submissions.rs`).
//! A submission is one query string that psql sends: the text of one query
//! buffer, with psql's own rewrites (`\;` and a send command become `;`, and
//! a variable becomes its value). Only a submission of server SQL is a
//! statement; the others (an empty query, a variable that is not set, an
//! unknown meta-command) are not.

use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)]
#[path = "../../../pgtk-sql/tests/support/submissions.rs"]
mod frozen;

use frozen::{FrozenSubmissions, SubmissionKind};

/// The PostgreSQL commit of the frozen corpus.
const GITLINK: &str = "6d396980fc5aed4f1a525e0bd75cb16b25ed40ca";

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
            "vendor/postgres does not hold the commit {commit} of the frozen corpus \
             ({error}); run `git submodule update --init` with full history"
        );
    }
}

/// A file of the PostgreSQL commit of the frozen corpus, such as
/// `src/test/regress/parallel_schedule`.
pub fn frozen_file(path: &str) -> String {
    require_commit(GITLINK);
    let spec = format!("{GITLINK}:{path}");
    let bytes = git(&["cat-file", "-p", &spec])
        .unwrap_or_else(|error| panic!("cannot read {spec} from vendor/postgres: {error}"));
    String::from_utf8(bytes).unwrap_or_else(|_| panic!("{spec} is not UTF-8"))
}

/// What one item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// A submission of server SQL.
    Statement,
    /// A submission that is not server SQL: an empty query, an interpolation
    /// of a variable that is not set, or an unknown meta-command.
    Other,
}

/// One submission of one corpus file.
#[derive(Clone, Debug)]
pub struct Item {
    /// The extent in the psql source.
    pub range: std::ops::Range<usize>,
    /// What the frozen submissions say the item is.
    pub kind: ItemKind,
    /// The query string that psql sends.
    pub text: String,
}

/// One corpus file with its submissions.
#[derive(Debug)]
pub struct CorpusFile {
    /// The file name, such as `int4.sql`.
    pub name: String,
    /// The submissions in source order.
    pub items: Vec<Item>,
}

impl CorpusFile {
    /// The text of one item: what psql sends.
    pub fn text_of<'a>(&self, item: &'a Item) -> &'a str {
        &item.text
    }
}

/// The frozen submissions of every corpus file.
pub struct FrozenSplit {
    submissions: &'static FrozenSubmissions,
}

impl FrozenSplit {
    /// Read the frozen submissions, after checking that the vendored
    /// PostgreSQL holds the commit of the corpus.
    pub fn load() -> Self {
        require_commit(GITLINK);
        Self {
            submissions: FrozenSubmissions::pinned(),
        }
    }

    /// The PostgreSQL commit of the corpus.
    pub fn gitlink(&self) -> &str {
        GITLINK
    }

    /// Every file name in the corpus.
    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.submissions.file_names().into_iter()
    }

    /// Read one corpus file, by its frozen blob ID, with its submissions.
    pub fn read(&self, name: &str) -> CorpusFile {
        let file = self.submissions.file(name);
        let source = file.source();
        let items = file
            .submissions(&source)
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .into_iter()
            .map(|submission| Item {
                range: submission.range,
                kind: if submission.kind == SubmissionKind::Server {
                    ItemKind::Statement
                } else {
                    ItemKind::Other
                },
                text: submission.text.into_owned(),
            })
            .collect();
        CorpusFile {
            name: name.to_owned(),
            items,
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
