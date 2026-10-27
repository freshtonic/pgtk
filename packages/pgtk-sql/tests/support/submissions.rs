//! The psql submissions of the frozen PostgreSQL 17.9 regression corpus, and
//! the version baseline of each target version over them (issue #10).
//!
//! `baselines/postgresql-17.9-submissions.json` records each query string that
//! psql sends to the server for each corpus file: its source extent, its kind
//! and the rewrites inside it. `pgtk-psql/tests/submissions.rs` derives the
//! record from the corpus and requires the same bytes. The corpus files are
//! read by Git blob ID from the `vendor/postgres` object database.
//!
//! This module needs only `std` and `serde_json`, and it does not name
//! `pgtk_sql`: `pgtk-oracle/tests/corpus_outcomes.rs` also mounts it, to
//! record the oracle outcomes of a version without a pgtk-sql build of that
//! version. So it keeps to edition 2021.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::OnceLock;

const PINNED_SUBMISSIONS: &str = include_str!("../../baselines/postgresql-17.9-submissions.json");
/// The temporary list of the over-acceptances that the strict check of #95
/// found (`docs/differential-baseline.md`).
const PENDING_OVER_ACCEPTANCES: &str = include_str!("../../baselines/pending-over-acceptance.json");
/// One version baseline for each target version (`docs/differential-baseline.md`).
const VERSION_BASELINES: &[(&str, &str)] = &[
    (
        "pg14",
        include_str!("../../baselines/target-versions/pg14.json"),
    ),
    (
        "pg15",
        include_str!("../../baselines/target-versions/pg15.json"),
    ),
    (
        "pg16",
        include_str!("../../baselines/target-versions/pg16.json"),
    ),
    (
        "pg17",
        include_str!("../../baselines/target-versions/pg17.json"),
    ),
    (
        "pg18",
        include_str!("../../baselines/target-versions/pg18.json"),
    ),
    (
        "pg19-beta",
        include_str!("../../baselines/target-versions/pg19-beta.json"),
    ),
];
/// The pin table of the oracle: `feature<TAB>ref<TAB>commit`.
const ORACLE_PINS: &str = include_str!("../../../pgtk-oracle/pins.tsv");
const POSTGRES_GITLINK: &str = "6d396980fc5aed4f1a525e0bd75cb16b25ed40ca";
const CORPUS_NAME: &str = "postgresql-17.9";

/// What one submission is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubmissionKind {
    /// Server SQL: the differential checks it.
    Server,
    /// No SQL token but `;`: the server has nothing to parse.
    Empty,
    /// An interpolation of a variable that is not set. Its value comes from
    /// the execution (`\gset`, `\getenv`) or from the server, which pgtk-psql
    /// does not model.
    Unbound,
    /// An unknown meta-command: psql reports it and sends none of it, and
    /// pgtk-psql keeps it in the text (pgtk-sql ADR 0013).
    UnknownCommand,
}

impl SubmissionKind {
    fn parse(code: u8) -> Self {
        match code {
            b'S' => Self::Server,
            b'E' => Self::Empty,
            b'U' => Self::Unbound,
            b'C' => Self::UnknownCommand,
            _ => panic!("invalid submission kind {:?}", code as char),
        }
    }

    /// The stable reason why the differential does not check a submission
    /// of this kind, or `None` for server SQL.
    pub fn exclusion(self) -> Option<&'static str> {
        match self {
            Self::Server => None,
            Self::Empty => Some("empty-query"),
            Self::Unbound => Some("unbound-variable"),
            Self::UnknownCommand => Some("unknown-meta-command"),
        }
    }
}

/// One frozen submission of a corpus file.
#[derive(Clone, Debug)]
pub struct FrozenSubmission<'source> {
    /// The extent in the psql source.
    pub range: Range<usize>,
    pub kind: SubmissionKind,
    /// The query string that psql sends.
    pub text: Cow<'source, str>,
}

/// One rewrite inside a submission: psql sends `text` in place of the
/// source extent `source`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Rewrite {
    source: Range<usize>,
    text: String,
}

/// The frozen submissions of one corpus file.
#[derive(Debug)]
pub struct SubmissionFile {
    pub source_git_blob: String,
    pub source_bytes: usize,
    ranges: Vec<Range<usize>>,
    kinds: Vec<SubmissionKind>,
    /// The rewrites of each submission, by submission index.
    rewrites: BTreeMap<usize, Vec<Rewrite>>,
}

impl SubmissionFile {
    /// The frozen corpus source, read by its Git blob ID from the
    /// `vendor/postgres` object database. The pinned oracle release can be
    /// later than the corpus release, so the checked-out file can differ.
    pub fn source(&self) -> String {
        let repository = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor/postgres");
        let output = std::process::Command::new("git")
            .args(["-C", repository, "cat-file", "blob", &self.source_git_blob])
            .output()
            .unwrap_or_else(|error| panic!("cannot run git cat-file: {error}"));
        assert!(
            output.status.success(),
            "cannot read frozen corpus blob {} from {repository} \
             (is the submodule checked out with full history?): {}",
            self.source_git_blob,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap_or_else(|_| panic!("frozen corpus blob {} is not UTF-8", self.source_git_blob))
    }

    /// The submissions of `source`, the text of this file.
    pub fn submissions<'source>(
        &self,
        source: &'source str,
    ) -> Result<Vec<FrozenSubmission<'source>>, String> {
        if source.len() != self.source_bytes {
            return Err(format!(
                "the source has {} bytes; the frozen source has {}",
                source.len(),
                self.source_bytes
            ));
        }
        let mut submissions = Vec::with_capacity(self.ranges.len());
        for (index, (range, kind)) in self.ranges.iter().zip(&self.kinds).enumerate() {
            let slice = |range: Range<usize>| {
                source.get(range.clone()).ok_or_else(|| {
                    format!("invalid UTF-8 byte range {}:{}", range.start, range.end)
                })
            };
            let text = match self.rewrites.get(&index) {
                None => Cow::Borrowed(slice(range.clone())?),
                Some(rewrites) => {
                    let mut text = String::new();
                    let mut cursor = range.start;
                    for rewrite in rewrites {
                        text.push_str(slice(cursor..rewrite.source.start)?);
                        text.push_str(&rewrite.text);
                        cursor = rewrite.source.end;
                    }
                    text.push_str(slice(cursor..range.end)?);
                    Cow::Owned(text)
                }
            };
            submissions.push(FrozenSubmission {
                range: range.clone(),
                kind: *kind,
                text,
            });
        }
        Ok(submissions)
    }

    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }

    pub fn kinds(&self) -> &[SubmissionKind] {
        &self.kinds
    }
}

/// The frozen submissions of every corpus file.
#[derive(Debug)]
pub struct FrozenSubmissions {
    files: BTreeMap<String, SubmissionFile>,
    total: usize,
}

impl FrozenSubmissions {
    pub fn pinned() -> &'static Self {
        static SUBMISSIONS: OnceLock<FrozenSubmissions> = OnceLock::new();
        SUBMISSIONS.get_or_init(|| Self::load(PINNED_SUBMISSIONS))
    }

    fn load(source: &str) -> Self {
        let document: serde_json::Value =
            serde_json::from_str(source).expect("parse the frozen submissions");
        assert_eq!(document["schema_version"].as_u64(), Some(1));
        assert_eq!(document["name"].as_str(), Some(CORPUS_NAME));
        assert_eq!(document["postgres"]["release"].as_str(), Some("17.9"));
        assert_eq!(
            document["postgres"]["gitlink"].as_str(),
            Some(POSTGRES_GITLINK)
        );
        assert_eq!(
            document["corpus_root"].as_str(),
            Some("vendor/postgres/src/test/regress/sql")
        );
        assert_eq!(
            document["encoding"].as_str(),
            Some("comma-separated-byte-ranges-v1")
        );
        assert_eq!(document["derivation"]["psql"].as_str(), Some("pg17"));

        let mut kind_totals = BTreeMap::<SubmissionKind, usize>::new();
        let files = document["files"]
            .as_array()
            .expect("submission files")
            .iter()
            .map(|file| {
                let name = file["file"].as_str().expect("file name").to_owned();
                let source_git_blob = file["source_git_blob"]
                    .as_str()
                    .expect("source Git blob")
                    .to_owned();
                assert!(
                    source_git_blob.len() == 40
                        && source_git_blob.bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "invalid source Git blob for {name}"
                );
                let source_bytes = as_usize(&file["source_bytes"], "source bytes");
                let count = as_usize(&file["submission_count"], "submission count");
                let ranges = parse_ranges(file["byte_ranges"].as_str().expect("byte ranges"));
                let kinds = file["kinds"]
                    .as_str()
                    .expect("submission kinds")
                    .bytes()
                    .map(SubmissionKind::parse)
                    .collect::<Vec<_>>();
                assert_eq!(ranges.len(), count, "submission count of {name}");
                assert_eq!(kinds.len(), count, "kind count of {name}");
                for kind in &kinds {
                    *kind_totals.entry(*kind).or_default() += 1;
                }
                let mut previous_end = 0;
                for range in &ranges {
                    assert!(
                        range.start < range.end
                            && range.start >= previous_end
                            && range.end <= source_bytes,
                        "invalid submission range {}:{} in {name}",
                        range.start,
                        range.end
                    );
                    previous_end = range.end;
                }
                let mut rewrites = BTreeMap::<usize, Vec<Rewrite>>::new();
                for rewrite in file["rewrites"].as_array().expect("rewrites") {
                    let index = as_usize(&rewrite[0], "rewrite submission index");
                    let source = parse_range(rewrite[1].as_str().expect("rewrite extent"));
                    let text = rewrite[2].as_str().expect("rewrite text").to_owned();
                    let range = ranges
                        .get(index)
                        .unwrap_or_else(|| panic!("{name}: rewrite of submission {index}"));
                    let entries = rewrites.entry(index).or_default();
                    let cursor = entries.last().map_or(range.start, |last| last.source.end);
                    assert!(
                        source.start >= cursor && source.end <= range.end,
                        "{name}: rewrite {}:{} is outside submission {index} or out of order",
                        source.start,
                        source.end
                    );
                    entries.push(Rewrite { source, text });
                }
                (
                    name,
                    SubmissionFile {
                        source_git_blob,
                        source_bytes,
                        ranges,
                        kinds,
                        rewrites,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let total = files.values().map(|file| file.ranges.len()).sum::<usize>();
        let totals = &document["totals"];
        assert_eq!(as_usize(&totals["files"], "file total"), files.len());
        assert_eq!(as_usize(&totals["submissions"], "submission total"), total);
        for (code, kind) in [
            ("S", SubmissionKind::Server),
            ("E", SubmissionKind::Empty),
            ("U", SubmissionKind::Unbound),
            ("C", SubmissionKind::UnknownCommand),
        ] {
            assert_eq!(
                as_usize(&totals[code], "kind total"),
                kind_totals.get(&kind).copied().unwrap_or(0),
                "total of the submissions of kind {code}"
            );
        }
        Self { files, total }
    }

    pub fn file(&self, name: &str) -> &SubmissionFile {
        self.files
            .get(name)
            .unwrap_or_else(|| panic!("{name} is absent from the frozen submissions"))
    }

    pub fn file_names(&self) -> BTreeSet<&str> {
        self.files.keys().map(String::as_str).collect()
    }

    pub fn total(&self) -> usize {
        self.total
    }

    /// The number of submissions of each kind.
    pub fn kind_totals(&self) -> BTreeMap<SubmissionKind, usize> {
        let mut totals = BTreeMap::new();
        for kind in self.files.values().flat_map(|file| &file.kinds) {
            *totals.entry(*kind).or_default() += 1;
        }
        totals
    }
}

/// One submission that the oracle rejects and pgtk-sql accepts, with the
/// cause. The grammar fix of the cause removes the entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingOverAcceptance {
    pub file: String,
    pub submission_index: usize,
    pub byte_range: Range<usize>,
    pub cause: String,
}

/// The temporary list of known over-acceptances (#95). Each entry applies
/// to a target version whose oracle rejects the submission. The differential
/// test then requires the over-acceptance failure, so a fix that is not
/// recorded and a new over-acceptance both fail.
#[derive(Debug)]
pub struct PendingOverAcceptances {
    causes: BTreeMap<String, String>,
    entries: Vec<PendingOverAcceptance>,
}

impl PendingOverAcceptances {
    pub fn pinned() -> &'static Self {
        static PENDING: OnceLock<PendingOverAcceptances> = OnceLock::new();
        PENDING.get_or_init(|| Self::load(PENDING_OVER_ACCEPTANCES))
    }

    fn load(source: &str) -> Self {
        let document: serde_json::Value =
            serde_json::from_str(source).expect("parse the pending over-acceptance list");
        assert_eq!(document["schema_version"].as_u64(), Some(2));
        assert_eq!(document["name"].as_str(), Some(CORPUS_NAME));
        let causes = document["causes"]
            .as_object()
            .expect("pending over-acceptance causes")
            .iter()
            .map(|(name, reason)| {
                (
                    name.clone(),
                    reason.as_str().expect("cause reason").to_owned(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let frozen = FrozenSubmissions::pinned();
        let mut previous: Option<(String, usize)> = None;
        let entries = document["submissions"]
            .as_array()
            .expect("pending over-acceptance submissions")
            .iter()
            .map(|entry| {
                let file = entry["file"].as_str().expect("entry file").to_owned();
                let submission_index =
                    as_usize(&entry["submission_index"], "entry submission index");
                let byte_range =
                    parse_range(entry["byte_range"].as_str().expect("entry byte range"));
                let cause = entry["cause"].as_str().expect("entry cause").to_owned();
                let identity = (file.clone(), submission_index);
                assert!(
                    previous
                        .as_ref()
                        .is_none_or(|previous| previous < &identity),
                    "pending over-acceptances are not unique and in file/index order at \
                     {file}:{submission_index}"
                );
                previous = Some(identity);
                assert_eq!(
                    frozen.file(&file).ranges().get(submission_index),
                    Some(&byte_range),
                    "pending over-acceptance {file}:{submission_index} byte range"
                );
                assert!(
                    causes.contains_key(&cause),
                    "pending over-acceptance {file}:{submission_index} has an unknown cause {cause:?}"
                );
                PendingOverAcceptance {
                    file,
                    submission_index,
                    byte_range,
                    cause,
                }
            })
            .collect::<Vec<_>>();
        let used = entries
            .iter()
            .map(|entry| entry.cause.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            used,
            causes.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            "each cause of the pending over-acceptance list has an entry"
        );
        Self { causes, entries }
    }

    pub fn entries(&self) -> &[PendingOverAcceptance] {
        &self.entries
    }

    /// The reason of `cause`.
    pub fn cause(&self, cause: &str) -> &str {
        &self.causes[cause]
    }

    /// The entries of `file`, by submission index.
    pub fn entries_in(&self, file: &str) -> BTreeMap<usize, &PendingOverAcceptance> {
        self.entries
            .iter()
            .filter(|entry| entry.file == file)
            .map(|entry| (entry.submission_index, entry))
            .collect()
    }
}

/// The pgtk-sql outcome of a submission in an expected version gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GapOutcome {
    /// pgtk-sql cannot parse a submission that the oracle accepts.
    Skip,
    /// The differential check fails: pgtk-sql over-accepts, or its output
    /// changes the parse tree or is rejected.
    Fail,
}

impl GapOutcome {
    pub fn name(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::Fail => "fail",
        }
    }
}

/// One submission where a build of the target version does not pass the
/// differential check with that version's oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionGap {
    pub file: String,
    pub submission_index: usize,
    pub byte_range: Range<usize>,
    pub pg_sql: GapOutcome,
    pub oracle_accepts: bool,
}

/// The oracle outcomes and the expected outcome counts of a version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutcomeTotals {
    /// The submissions that the differential checks (kind `S`).
    pub checked: usize,
    pub pass: usize,
    pub skip: usize,
    pub fail: usize,
}

/// The differential baseline of one target version: the oracle outcome of
/// every frozen server submission, and the expected version gaps.
#[derive(Debug)]
pub struct VersionBaseline {
    pub feature: String,
    pub oracle_ref: String,
    pub oracle_commit: String,
    /// `None` for a submission that the differential does not check.
    oracle_accepts: BTreeMap<String, Vec<Option<bool>>>,
    gaps: Vec<VersionGap>,
}

/// The pin of `feature` in `pgtk-oracle/pins.tsv`: `(ref, commit)`.
pub fn oracle_pin(feature: &str) -> (&'static str, &'static str) {
    ORACLE_PINS
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 3, "pins.tsv row {line:?}");
            (fields[0], fields[1], fields[2])
        })
        .find(|(name, _, _)| *name == feature)
        .map(|(_, reference, commit)| (reference, commit))
        .unwrap_or_else(|| panic!("pins.tsv has no pin for {feature}"))
}

/// The version features that have a version baseline.
pub fn version_features() -> impl Iterator<Item = &'static str> {
    VERSION_BASELINES.iter().map(|(feature, _)| *feature)
}

impl VersionBaseline {
    /// The version baseline of the version feature `feature`.
    pub fn of(feature: &str) -> &'static Self {
        static BASELINES: OnceLock<BTreeMap<&'static str, VersionBaseline>> = OnceLock::new();
        BASELINES
            .get_or_init(|| {
                VERSION_BASELINES
                    .iter()
                    .map(|(feature, source)| (*feature, Self::load(feature, source)))
                    .collect()
            })
            .get(feature)
            .unwrap_or_else(|| panic!("no version baseline for {feature}"))
    }

    /// Reads the version baseline text `source` of `feature`.
    pub fn load(feature: &str, source: &str) -> Self {
        let document: serde_json::Value = serde_json::from_str(source)
            .unwrap_or_else(|error| panic!("parse the {feature} version baseline: {error}"));
        assert_eq!(document["schema_version"].as_u64(), Some(2), "{feature}");
        assert_eq!(document["target_version"].as_str(), Some(feature));
        assert_eq!(
            document["corpus"]["name"].as_str(),
            Some(CORPUS_NAME),
            "{feature}: the version baselines share the frozen corpus"
        );
        assert_eq!(
            document["corpus"]["gitlink"].as_str(),
            Some(POSTGRES_GITLINK),
            "{feature}"
        );
        assert_eq!(
            document["corpus"]["submissions"].as_str(),
            Some("postgresql-17.9-submissions.json"),
            "{feature}: the statement set is the frozen psql submissions"
        );
        let (pin_ref, pin_commit) = oracle_pin(feature);
        let oracle_ref = document["oracle"]["ref"]
            .as_str()
            .expect("oracle ref")
            .to_owned();
        let oracle_commit = document["oracle"]["commit"]
            .as_str()
            .expect("oracle commit")
            .to_owned();
        assert_eq!(
            (oracle_ref.as_str(), oracle_commit.as_str()),
            (pin_ref, pin_commit),
            "{feature}: the version baseline is not from the pinned oracle; regenerate it"
        );

        let frozen = FrozenSubmissions::pinned();
        let oracle_accepts = document["files"]
            .as_array()
            .expect("version baseline files")
            .iter()
            .map(|file| {
                let name = file["file"].as_str().expect("file name").to_owned();
                let kinds = frozen.file(&name).kinds();
                let accepts = file["oracle_accepts"]
                    .as_str()
                    .expect("oracle outcomes")
                    .bytes()
                    .map(|outcome| match outcome {
                        b'A' => Some(true),
                        b'R' => Some(false),
                        b'-' => None,
                        _ => panic!("{feature}: invalid oracle outcome {outcome:?} in {name}"),
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    accepts.len(),
                    kinds.len(),
                    "{feature}: oracle outcome count for {name}"
                );
                for (index, (accepts, kind)) in accepts.iter().zip(kinds).enumerate() {
                    assert_eq!(
                        accepts.is_some(),
                        *kind == SubmissionKind::Server,
                        "{feature}: {name}:{index} has an oracle outcome exactly when it is \
                         server SQL"
                    );
                }
                (name, accepts)
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            oracle_accepts
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            frozen.file_names(),
            "{feature}: the version baseline covers every frozen corpus file"
        );

        let mut previous: Option<(String, usize)> = None;
        let gaps = document["expected_version_gaps"]
            .as_array()
            .expect("expected version gaps")
            .iter()
            .map(|gap| {
                let file = gap["file"].as_str().expect("gap file").to_owned();
                let submission_index = as_usize(&gap["submission_index"], "gap submission index");
                let byte_range = parse_range(gap["byte_range"].as_str().expect("gap byte range"));
                let identity = (file.clone(), submission_index);
                assert!(
                    previous
                        .as_ref()
                        .is_none_or(|previous| previous < &identity),
                    "{feature}: version gaps are not unique and in file/index order at \
                     {file}:{submission_index}"
                );
                previous = Some(identity);
                assert_eq!(
                    frozen.file(&file).ranges().get(submission_index),
                    Some(&byte_range),
                    "{feature}: version gap {file}:{submission_index} byte range"
                );
                let pg_sql = match gap["pg_sql"].as_str().expect("gap pg_sql outcome") {
                    "skip" => GapOutcome::Skip,
                    "fail" => GapOutcome::Fail,
                    other => panic!("{feature}: invalid gap pg_sql outcome {other:?}"),
                };
                let gap_oracle_accepts = match gap["oracle"].as_str().expect("gap oracle outcome") {
                    "accept" => true,
                    "reject" => false,
                    other => panic!("{feature}: invalid gap oracle outcome {other:?}"),
                };
                assert_eq!(
                    Some(gap_oracle_accepts),
                    oracle_accepts[&file][submission_index],
                    "{feature}: version gap {file}:{submission_index} oracle outcome"
                );
                assert!(
                    pg_sql != GapOutcome::Skip || gap_oracle_accepts,
                    "{feature}: a skip gap is a submission that the oracle accepts \
                     ({file}:{submission_index})"
                );
                VersionGap {
                    file,
                    submission_index,
                    byte_range,
                    pg_sql,
                    oracle_accepts: gap_oracle_accepts,
                }
            })
            .collect::<Vec<_>>();

        let baseline = Self {
            feature: feature.to_owned(),
            oracle_ref,
            oracle_commit,
            oracle_accepts,
            gaps,
        };
        let totals = &document["totals"];
        let expected = baseline.expected_totals();
        let kind_totals = frozen.kind_totals();
        assert_eq!(
            (
                as_usize(&totals["submissions"], "total submissions"),
                as_usize(&totals["checked"], "total checked"),
                as_usize(&totals["excluded"], "total excluded"),
                as_usize(&totals["oracle_accepts"], "total oracle accepts"),
                as_usize(&totals["pass"], "total pass"),
                as_usize(&totals["skip"], "total skip"),
                as_usize(&totals["fail"], "total fail"),
                as_usize(&totals["expected_version_gaps"], "total gaps"),
            ),
            (
                frozen.total(),
                expected.checked,
                frozen.total() - expected.checked,
                baseline.total_oracle_accepts(),
                expected.pass,
                expected.skip,
                expected.fail,
                baseline.gaps.len(),
            ),
            "{feature}: version baseline totals"
        );
        assert_eq!(
            expected.checked,
            kind_totals
                .get(&SubmissionKind::Server)
                .copied()
                .unwrap_or(0)
        );
        baseline
    }

    /// The oracle outcome of every submission of `file`: `None` for a
    /// submission that the differential does not check.
    pub fn oracle_accepts(&self, file: &str) -> &[Option<bool>] {
        self.oracle_accepts
            .get(file)
            .unwrap_or_else(|| panic!("{file} is absent from the {} baseline", self.feature))
    }

    pub fn gaps(&self) -> &[VersionGap] {
        &self.gaps
    }

    /// The expected version gaps of `file`, by submission index.
    pub fn gaps_in(&self, file: &str) -> BTreeMap<usize, &VersionGap> {
        self.gaps
            .iter()
            .filter(|gap| gap.file == file)
            .map(|gap| (gap.submission_index, gap))
            .collect()
    }

    fn total_oracle_accepts(&self) -> usize {
        self.oracle_accepts
            .values()
            .flatten()
            .filter(|accepts| **accepts == Some(true))
            .count()
    }

    /// The expected outcome counts: every checked submission passes but the
    /// expected version gaps.
    pub fn expected_totals(&self) -> OutcomeTotals {
        expected_totals(&self.oracle_accepts, &self.gaps)
    }
}

fn expected_totals(
    oracle_accepts: &BTreeMap<String, Vec<Option<bool>>>,
    gaps: &[VersionGap],
) -> OutcomeTotals {
    let checked = oracle_accepts
        .values()
        .flatten()
        .filter(|accepts| accepts.is_some())
        .count();
    let skip = gaps
        .iter()
        .filter(|gap| gap.pg_sql == GapOutcome::Skip)
        .count();
    let fail = gaps.len() - skip;
    OutcomeTotals {
        checked,
        pass: checked - gaps.len(),
        skip,
        fail,
    }
}

/// Render a version baseline file. The regeneration tests write it; see
/// `docs/differential-baseline.md`.
pub fn render_version_baseline(
    feature: &str,
    oracle_accepts: &BTreeMap<String, Vec<Option<bool>>>,
    gaps: &[VersionGap],
) -> String {
    let (reference, commit) = oracle_pin(feature);
    let frozen = FrozenSubmissions::pinned();
    let totals = expected_totals(oracle_accepts, gaps);
    // One gap and one file on each line, so a change is a small diff.
    let line = |value: serde_json::Value| serde_json::to_string(&value).expect("render JSON");
    let gaps = gaps
        .iter()
        .map(|gap| {
            line(serde_json::json!({
                "file": gap.file,
                "submission_index": gap.submission_index,
                "byte_range": format!("{}:{}", gap.byte_range.start, gap.byte_range.end),
                "pg_sql": gap.pg_sql.name(),
                "oracle": if gap.oracle_accepts { "accept" } else { "reject" },
            }))
        })
        .collect::<Vec<_>>();
    let files = oracle_accepts
        .iter()
        .map(|(file, accepts)| {
            line(serde_json::json!({
                "file": file,
                "oracle_accepts": accepts
                    .iter()
                    .map(|accepts| match accepts {
                        Some(true) => 'A',
                        Some(false) => 'R',
                        None => '-',
                    })
                    .collect::<String>(),
            }))
        })
        .collect::<Vec<_>>();
    let list = |items: Vec<String>| {
        if items.is_empty() {
            "[]".to_owned()
        } else {
            format!("[\n    {}\n  ]", items.join(",\n    "))
        }
    };
    format!(
        "{{\n  \"schema_version\": 2,\n  \"target_version\": {},\n  \"oracle\": {},\n  \
         \"corpus\": {},\n  \"totals\": {},\n  \"expected_version_gaps\": {},\n  \
         \"files\": {}\n}}\n",
        line(serde_json::json!(feature)),
        line(serde_json::json!({ "ref": reference, "commit": commit })),
        line(serde_json::json!({
            "name": CORPUS_NAME,
            "gitlink": POSTGRES_GITLINK,
            "submissions": "postgresql-17.9-submissions.json",
        })),
        line(serde_json::json!({
            "submissions": frozen.total(),
            "checked": totals.checked,
            "excluded": frozen.total() - totals.checked,
            "oracle_accepts": oracle_accepts
                .values()
                .flatten()
                .filter(|accepts| **accepts == Some(true))
                .count(),
            "pass": totals.pass,
            "skip": totals.skip,
            "fail": totals.fail,
            "expected_version_gaps": gaps.len(),
        })),
        list(gaps),
        list(files),
    )
}

fn as_usize(value: &serde_json::Value, field: &str) -> usize {
    value
        .as_u64()
        .unwrap_or_else(|| panic!("baseline {field} must be an unsigned integer"))
        .try_into()
        .expect("baseline count fits usize")
}

fn parse_ranges(encoded: &str) -> Vec<Range<usize>> {
    if encoded.is_empty() {
        return Vec::new();
    }
    encoded.split(',').map(parse_range).collect()
}

pub fn parse_range(encoded: &str) -> Range<usize> {
    let (start, end) = encoded.split_once(':').expect("encoded byte range");
    start.parse().expect("byte-range start")..end.parse().expect("byte-range end")
}
