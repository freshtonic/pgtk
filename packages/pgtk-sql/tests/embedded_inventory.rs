//! Exact accounting for test items relocated outside Recursa discovery.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

type InventoryRow = (String, String, bool);

/// The oldest and the newest PostgreSQL major that pgtk-sql can target. The
/// `pg19-beta` target version is major 19.
const OLDEST_TARGET_MAJOR: u32 = 14;
const NEWEST_TARGET_MAJOR: u32 = 19;

/// The embedded tests that run for each target version: the inventory rows
/// whose version range contains that major. Bump the count of each version
/// that a new test runs for.
const ROWS_PER_TARGET_VERSION: [(u32, usize); 6] = [
    (14, 1_212),
    (15, 1_233),
    (16, 1_255),
    (17, 1_266),
    (18, 1_273),
    (19, 1_275),
];

/// The target versions that one embedded test runs for, written `LO-` (from
/// major LO, with no upper limit) or `LO-HI` (majors LO to HI, both
/// included). LO is never below 14. A test without a version gate is `14-`.
/// `feature = "since-pgN"` makes LO at least N, and
/// `not(feature = "since-pgN")` makes HI at most N - 1.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct VersionRange {
    oldest: u32,
    newest: Option<u32>,
}

impl VersionRange {
    const ALL: Self = Self {
        oldest: OLDEST_TARGET_MAJOR,
        newest: None,
    };

    fn parse(text: &str) -> Result<Self, String> {
        let (oldest, newest) = text
            .split_once('-')
            .ok_or_else(|| format!("version range {text:?} is not `LO-` or `LO-HI`"))?;
        let major = |value: &str| {
            value
                .parse::<u32>()
                .ok()
                .filter(|major| (OLDEST_TARGET_MAJOR..=NEWEST_TARGET_MAJOR).contains(major))
                .ok_or_else(|| format!("version range {text:?} has an unknown major {value:?}"))
        };
        let range = Self {
            oldest: major(oldest)?,
            newest: if newest.is_empty() {
                None
            } else {
                Some(major(newest)?)
            },
        };
        if range.newest.is_some_and(|newest| newest < range.oldest) {
            return Err(format!("version range {text:?} is empty"));
        }
        if range.newest == Some(NEWEST_TARGET_MAJOR) {
            return Err(format!(
                "version range {text:?} must be written `{}-`",
                range.oldest
            ));
        }
        Ok(range)
    }

    fn contains(self, major: u32) -> bool {
        self.oldest <= major && self.newest.is_none_or(|newest| major <= newest)
    }

    fn intersect(self, other: Self) -> Self {
        Self {
            oldest: self.oldest.max(other.oldest),
            newest: match (self.newest, other.newest) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            },
        }
    }
}

impl std::fmt::Display for VersionRange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.newest {
            Some(newest) => write!(formatter, "{}-{newest}", self.oldest),
            None => write!(formatter, "{}-", self.oldest),
        }
    }
}

type VersionRanges = BTreeMap<(String, String), VersionRange>;

fn inventory_rows() -> Vec<(InventoryRow, VersionRange)> {
    include_str!("../embedded-tests/inventory.tsv")
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let mut fields = line.split('\t');
            let path = fields.next().expect("inventory path").to_owned();
            let name = fields.next().expect("inventory test name").to_owned();
            let ignored = fields
                .next()
                .expect("inventory ignored status")
                .parse()
                .expect("boolean ignored status");
            let range = VersionRange::parse(fields.next().expect("inventory version range"))
                .unwrap_or_else(|error| panic!("inventory line {}: {error}", index + 1));
            assert!(fields.next().is_none(), "four inventory fields");
            ((path, name, ignored), range)
        })
        .collect()
}

fn expected_inventory() -> BTreeSet<InventoryRow> {
    let mut inventory = BTreeSet::new();
    for (index, (row, _)) in inventory_rows().into_iter().enumerate() {
        assert!(
            inventory.insert(row),
            "duplicate embedded-test inventory row at line {}",
            index + 1
        );
    }
    inventory
}

fn expected_version_ranges() -> VersionRanges {
    inventory_rows()
        .into_iter()
        .map(|((path, name, _), range)| ((path, name), range))
        .collect()
}

/// The version range that a `cfg` predicate gives. A predicate that does
/// not name a version feature gives every version.
fn cfg_version_range(predicate: &syn::Meta) -> VersionRange {
    let since = |meta: &syn::Meta| -> Option<u32> {
        let syn::Meta::NameValue(pair) = meta else {
            return None;
        };
        if !pair.path.is_ident("feature") {
            return None;
        }
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(feature),
            ..
        }) = &pair.value
        else {
            return None;
        };
        let feature = feature.value();
        if let Some(major) = feature.strip_prefix("since-pg") {
            return Some(major.parse().expect("since-pgN names a major version"));
        }
        assert!(
            !(feature.starts_with("pg") && feature[2..].starts_with(char::is_numeric)),
            "gate an embedded test with `since-pgN`, not with the version feature {feature:?}"
        );
        None
    };
    let nested = |list: &syn::MetaList| -> Vec<syn::Meta> {
        list.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )
        .expect("cfg predicate list")
        .into_iter()
        .collect()
    };
    let names_version = |meta: &syn::Meta| cfg_version_range(meta) != VersionRange::ALL;

    if let Some(major) = since(predicate) {
        return VersionRange {
            oldest: major,
            newest: None,
        };
    }
    let syn::Meta::List(list) = predicate else {
        return VersionRange::ALL;
    };
    if list.path.is_ident("all") {
        return nested(list)
            .iter()
            .map(cfg_version_range)
            .fold(VersionRange::ALL, VersionRange::intersect);
    }
    if list.path.is_ident("not") {
        let inner = nested(list);
        if let [inner] = inner.as_slice()
            && let Some(major) = since(inner)
        {
            return VersionRange {
                oldest: OLDEST_TARGET_MAJOR,
                newest: Some(major - 1),
            };
        }
        assert!(
            !inner.iter().any(names_version),
            "an embedded-test version gate must be `since-pgN`, `not(since-pgN)` or `all(...)`"
        );
        return VersionRange::ALL;
    }
    assert!(
        !nested(list).iter().any(names_version),
        "an embedded-test version gate must be `since-pgN`, `not(since-pgN)` or `all(...)`"
    );
    VersionRange::ALL
}

fn attributes_version_range(
    attributes: &[syn::Attribute],
    inherited: VersionRange,
) -> VersionRange {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .map(|attribute| {
            cfg_version_range(&attribute.parse_args::<syn::Meta>().expect("cfg predicate"))
        })
        .fold(inherited, VersionRange::intersect)
}

/// The version range of each test function in one relocated test module,
/// from its `cfg` attributes and those of the modules around it.
fn test_version_ranges(root: &Path, path: &Path) -> VersionRanges {
    let inventory_path = path
        .strip_prefix(root)
        .expect("embedded test below repository root")
        .to_string_lossy()
        .into_owned();
    let source = fs::read_to_string(path).expect("read relocated test module");
    let parsed = syn::parse_file(&source).expect("parse relocated test module");
    let mut ranges = BTreeMap::new();
    let mut pending = vec![(parsed.items.as_slice(), VersionRange::ALL)];
    while let Some((items, inherited)) = pending.pop() {
        for item in items {
            if let syn::Item::Mod(module) = item
                && let Some((_, items)) = &module.content
            {
                pending.push((items, attributes_version_range(&module.attrs, inherited)));
            }
            if let syn::Item::Fn(function) = item
                && function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("test"))
            {
                ranges.insert(
                    (inventory_path.clone(), function.sig.ident.to_string()),
                    attributes_version_range(&function.attrs, inherited),
                );
            }
        }
    }
    ranges
}

fn rust_files_below(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![directory.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("read source directory") {
            let path = entry.expect("read source entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files
}

fn discovered_test_modules(root: &Path) -> BTreeSet<String> {
    rust_files_below(&root.join("embedded-tests"))
        .into_iter()
        .filter(|path| !test_rows(root, path).is_empty())
        .map(|path| {
            path.strip_prefix(root)
                .expect("embedded test below repository root")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn included_test_modules(root: &Path, discovered: &BTreeSet<String>) -> BTreeMap<String, usize> {
    let mut included = BTreeMap::new();
    for path in rust_files_below(&root.join("tests/embedded")) {
        let source = fs::read_to_string(&path).expect("read Rust source");
        let parsed = syn::parse_file(&source).expect("parse Rust source");
        let mut pending = vec![parsed.items.as_slice()];
        while let Some(items) = pending.pop() {
            for item in items {
                if let syn::Item::Mod(module) = item
                    && let Some((_, items)) = &module.content
                {
                    pending.push(items);
                }
                let syn::Item::Macro(item) = item else {
                    continue;
                };
                if !item.mac.path.is_ident("include") {
                    continue;
                }
                let arguments = item.mac.tokens.to_string();
                for test_path in discovered {
                    if arguments.contains(test_path) {
                        *included.entry(test_path.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    included
}

fn test_rows(root: &Path, path: &Path) -> BTreeSet<InventoryRow> {
    let inventory_path = path
        .strip_prefix(root)
        .expect("embedded test below repository root")
        .to_string_lossy()
        .into_owned();
    let source = fs::read_to_string(path).expect("read relocated test module");
    let parsed = syn::parse_file(&source).expect("parse relocated test module");
    let mut rows = BTreeSet::new();
    let mut pending = vec![parsed.items.as_slice()];
    while let Some(items) = pending.pop() {
        for item in items {
            if let syn::Item::Mod(module) = item
                && let Some((_, items)) = &module.content
            {
                pending.push(items);
            }
            if let syn::Item::Fn(function) = item {
                if !function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("test"))
                {
                    continue;
                }
                let ignored = function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("ignore"));
                rows.insert((
                    inventory_path.clone(),
                    function.sig.ident.to_string(),
                    ignored,
                ));
            }
        }
    }
    rows
}

fn actual_inventory(root: &Path, paths: &BTreeSet<String>) -> BTreeSet<InventoryRow> {
    let mut actual = BTreeSet::new();
    for path in paths {
        actual.extend(test_rows(root, &root.join(path)));
    }
    actual
}

#[test]
fn all_imported_embedded_tests_and_ignored_statuses_are_accounted_for() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let expected = expected_inventory();
    let expected_paths = expected
        .iter()
        .map(|(path, _, _)| path.clone())
        .collect::<BTreeSet<_>>();
    let discovered_paths = discovered_test_modules(root);
    let included_paths = included_test_modules(root, &discovered_paths);
    let actual = actual_inventory(root, &discovered_paths);

    assert_eq!(expected.len(), 1_335);
    assert_eq!(discovered_paths, expected_paths);
    assert_eq!(
        included_paths,
        discovered_paths
            .iter()
            .map(|path| (path.clone(), 1))
            .collect::<BTreeMap<_, _>>(),
        "every relocated test module must be included exactly once from the embedded target"
    );
    let missing = expected.difference(&actual).take(20).collect::<Vec<_>>();
    let unexpected = actual.difference(&expected).take(20).collect::<Vec<_>>();
    assert!(
        actual == expected,
        "embedded inventory mismatch: missing={missing:?}, unexpected={unexpected:?}"
    );
    assert_eq!(
        actual
            .iter()
            .filter(|(_, _, ignored)| *ignored)
            .map(|(_, name, _)| name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "parse_select_func_table_bare_alias_col_def",
            "report_ast_sizes",
        ])
    );
}

#[test]
fn each_version_range_agrees_with_the_version_gate_of_its_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let expected = expected_version_ranges();
    let mut actual = BTreeMap::new();
    for path in discovered_test_modules(root) {
        actual.extend(test_version_ranges(root, &root.join(path)));
    }
    let mismatches = expected
        .iter()
        .filter_map(|(test, range)| {
            let gate = actual.get(test).copied();
            (gate != Some(*range)).then(|| {
                format!(
                    "{}::{}: inventory {range}, cfg gate {}",
                    test.0,
                    test.1,
                    gate.map_or("absent".to_owned(), |gate| gate.to_string())
                )
            })
        })
        .collect::<Vec<_>>();
    assert!(
        mismatches.is_empty(),
        "embedded-test version ranges do not match their cfg gates:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn the_embedded_test_count_of_each_target_version_is_pinned() {
    let ranges = expected_version_ranges();
    let counts = (OLDEST_TARGET_MAJOR..=NEWEST_TARGET_MAJOR)
        .map(|major| {
            (
                major,
                ranges
                    .values()
                    .filter(|range| range.contains(major))
                    .count(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(counts, ROWS_PER_TARGET_VERSION);
    assert!(
        ROWS_PER_TARGET_VERSION
            .iter()
            .any(|(major, _)| *major == pgtk_sql::TARGET_VERSION.major()),
        "the target version of this build has a pinned count"
    );
}

#[test]
fn version_ranges_have_one_canonical_form() {
    assert_eq!(VersionRange::parse("14-"), Ok(VersionRange::ALL));
    assert_eq!(
        VersionRange::parse("15-16").map(|range| range.to_string()),
        Ok("15-16".to_owned())
    );
    assert!(VersionRange::parse("14-19").is_err());
    assert!(VersionRange::parse("13-").is_err());
    assert!(VersionRange::parse("-16").is_err());
    assert!(VersionRange::parse("17-16").is_err());
    assert!(VersionRange::parse("17").is_err());

    let gate = |source: &str| {
        let function: syn::ItemFn = syn::parse_str(source).expect("test function");
        attributes_version_range(&function.attrs, VersionRange::ALL).to_string()
    };
    assert_eq!(gate("#[test] fn t() {}"), "14-");
    assert_eq!(
        gate("#[cfg(feature = \"since-pg17\")] #[test] fn t() {}"),
        "17-"
    );
    assert_eq!(
        gate("#[cfg(not(feature = \"since-pg17\"))] #[test] fn t() {}"),
        "14-16"
    );
    assert_eq!(
        gate(
            "#[cfg(all(feature = \"since-pg15\", not(feature = \"since-pg18\")))] \
             #[test] fn t() {}"
        ),
        "15-17"
    );
    assert_eq!(gate("#[cfg(feature = \"spans\")] #[test] fn t() {}"), "14-");
}
