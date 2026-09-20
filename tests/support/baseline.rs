//! The pinned Oracle baseline: per-file counts that the run must reproduce
//! exactly. A better result fails the test too, until it is pinned on
//! purpose with `PG_ANALYZE_UPDATE_BASELINE=1`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::replay::Counts;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub struct FileCounts {
    pub items: usize,
    pub skipped: usize,
    pub skipped_not_a_statement: usize,
    pub skipped_parse_failed: usize,
    pub skipped_aborted_transaction: usize,
    pub skipped_catalog_unavailable: usize,
    pub unsupported: usize,
    pub agree: usize,
    pub disagree: usize,
}

impl FileCounts {
    const ZERO: Self = Self {
        items: 0,
        skipped: 0,
        skipped_not_a_statement: 0,
        skipped_parse_failed: 0,
        skipped_aborted_transaction: 0,
        skipped_catalog_unavailable: 0,
        unsupported: 0,
        agree: 0,
        disagree: 0,
    };

    fn add(self, other: &Self) -> Self {
        Self {
            items: self.items + other.items,
            skipped: self.skipped + other.skipped,
            skipped_not_a_statement: self.skipped_not_a_statement + other.skipped_not_a_statement,
            skipped_parse_failed: self.skipped_parse_failed + other.skipped_parse_failed,
            skipped_aborted_transaction: self.skipped_aborted_transaction
                + other.skipped_aborted_transaction,
            skipped_catalog_unavailable: self.skipped_catalog_unavailable
                + other.skipped_catalog_unavailable,
            unsupported: self.unsupported + other.unsupported,
            agree: self.agree + other.agree,
            disagree: self.disagree + other.disagree,
        }
    }
}

impl From<Counts> for FileCounts {
    fn from(counts: Counts) -> Self {
        Self {
            items: counts.items,
            skipped: counts.skipped,
            skipped_not_a_statement: counts.skipped_not_a_statement,
            skipped_parse_failed: counts.skipped_parse_failed,
            skipped_aborted_transaction: counts.skipped_aborted_transaction,
            skipped_catalog_unavailable: counts.skipped_catalog_unavailable,
            unsupported: counts.unsupported,
            agree: counts.agree,
            disagree: counts.disagree,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Baseline {
    pub schema_version: u32,
    pub postgres_major: u32,
    pub totals: FileCounts,
    pub files: BTreeMap<String, FileCounts>,
}

impl Baseline {
    pub fn path(postgres_major: u32) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("baselines")
            .join(format!("oracle-postgresql-{postgres_major}.json"))
    }

    pub fn from_results(postgres_major: u32, files: BTreeMap<String, FileCounts>) -> Self {
        let totals = files
            .values()
            .fold(FileCounts::ZERO, |acc, file| acc.add(file));
        Self {
            schema_version: 1,
            postgres_major,
            totals,
            files,
        }
    }

    pub fn load(postgres_major: u32) -> Option<Self> {
        let path = Self::path(postgres_major);
        let text = std::fs::read_to_string(&path).ok()?;
        Some(
            serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display())),
        )
    }

    pub fn save(&self) {
        let path = Self::path(self.postgres_major);
        std::fs::create_dir_all(path.parent().expect("baselines dir"))
            .expect("create baselines dir");
        let mut text = serde_json::to_string_pretty(self).expect("serialise baseline");
        text.push('\n');
        std::fs::write(&path, text)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
    }

    /// Every file whose counts differ, as `name: pinned -> actual` lines.
    pub fn differences(&self, actual: &Self) -> Vec<String> {
        let mut lines = Vec::new();
        for name in self.files.keys().chain(actual.files.keys()) {
            match (self.files.get(name), actual.files.get(name)) {
                (Some(pinned), Some(run)) if pinned == run => {}
                (pinned, run) => {
                    if lines
                        .iter()
                        .any(|line: &String| line.starts_with(&format!("{name}:")))
                    {
                        continue;
                    }
                    lines.push(format!("{name}: pinned {pinned:?} -> actual {run:?}"));
                }
            }
        }
        if self.totals != actual.totals {
            lines.push(format!(
                "totals: pinned {:?} -> actual {:?}",
                self.totals, actual.totals
            ));
        }
        lines
    }
}
