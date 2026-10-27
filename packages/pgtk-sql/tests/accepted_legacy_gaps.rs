//! Exact current outcomes for the PostgreSQL-accepted legacy grammar gaps.
//!
//! The ledger names statements by their legacy statement ranges, which are
//! no longer the statement set of the differential (issue #10). So this test
//! asks the live oracle about each legacy statement, and it finds an expected
//! version gap by the frozen psql submission that overlaps the statement.

mod support;

use std::collections::BTreeSet;
use std::ops::Range;

use pgtk_oracle::parse_ok;
use support::baseline::{
    AcceptedLegacyGapOutcome, AcceptedLegacyGaps, FrozenStatements, LegacyItemKind, VersionBaseline,
};
use support::diff_check::{Outcome, StrictDiagnostic, check_statement, pgsql_format};

/// The legacy statement `index` of `file`.
fn legacy_statement(file: &str, index: usize) -> String {
    let frozen = FrozenStatements::pinned().file(file);
    let source = frozen.source();
    frozen
        .statements(&source)
        .unwrap_or_else(|error| panic!("cannot load {file}: {error}"))[index]
        .to_owned()
}

#[test]
fn frozen_accepted_legacy_gap_contracts_are_exact() {
    let frozen = FrozenStatements::pinned();
    let gaps = AcceptedLegacyGaps::pinned();
    let mut derived_identities = BTreeSet::new();

    for file_name in frozen.file_names() {
        let file = frozen.file(file_name);
        let source = file.source();
        let statements = file
            .statements(&source)
            .unwrap_or_else(|error| panic!("cannot load {file_name}: {error}"));
        for (statement_index, (statement, kind)) in statements
            .into_iter()
            .zip(file.legacy_item_kinds())
            .enumerate()
        {
            if *kind == LegacyItemKind::ParseError && parse_ok(statement) {
                let range = &file.ranges()[statement_index];
                derived_identities.insert((
                    file_name.to_owned(),
                    statement_index,
                    range.start,
                    range.end,
                ));
            }
        }
    }

    let contracted_identities = gaps
        .entries()
        .iter()
        .map(|gap| {
            (
                gap.file.clone(),
                gap.statement_index,
                gap.byte_range.start,
                gap.byte_range.end,
            )
        })
        .collect::<BTreeSet<_>>();
    // The ledger is the contract of the PostgreSQL 17 oracle. The oracle of
    // another target version can reject a ledger statement, and it can
    // accept a legacy parse error that 17 rejects. A build of that version
    // then has the 17 grammar, so each such statement must be in an expected
    // version gap (docs/differential-baseline.md).
    let version = VersionBaseline::active();
    let in_version_gap = |file: &str, range: &Range<usize>| {
        version.gaps().iter().any(|gap| {
            gap.file == file && gap.byte_range.start < range.end && range.start < gap.byte_range.end
        })
    };
    let accepted_contracts = contracted_identities
        .iter()
        .filter(|(file, index, _, _)| parse_ok(&legacy_statement(file, *index)))
        .cloned()
        .collect::<BTreeSet<_>>();
    let rejected_contracts = contracted_identities
        .difference(&accepted_contracts)
        .collect::<Vec<_>>();
    assert!(
        version.feature != "pg17" || rejected_contracts.is_empty(),
        "the PostgreSQL 17 oracle rejects accepted legacy gaps: {rejected_contracts:?}"
    );
    let not_contracted = derived_identities
        .difference(&accepted_contracts)
        .filter(|(file, _, start, end)| !in_version_gap(file, &(*start..*end)))
        .collect::<Vec<_>>();
    assert!(
        version.feature != "pg17" || not_contracted.is_empty(),
        "legacy parse errors that the PostgreSQL 17 oracle accepts are not in the \
         accepted legacy gap ledger: {not_contracted:?}"
    );
    // Another version's oracle can accept a legacy parse error that 17
    // rejects. Unless it is in an expected version gap, pgtk-sql must pass it.
    for (file, index, _, _) in not_contracted {
        assert_eq!(
            check_statement(&support::Stmt {
                source: legacy_statement(file, *index),
            }),
            Outcome::Pass,
            "{file}:{index} is accepted by the {} oracle but is not in an expected version gap",
            version.feature
        );
    }
    assert!(
        accepted_contracts.is_subset(&derived_identities),
        "the {} oracle outcomes of the ledger disagree with the legacy item kinds",
        version.feature
    );

    for gap in gaps.entries() {
        let statement = legacy_statement(&gap.file, gap.statement_index);
        if !parse_ok(&statement) || in_version_gap(&gap.file, &gap.byte_range) {
            // The target version rejects the statement, or its build has an
            // expected version gap there. The differential test checks both.
            continue;
        }

        match &gap.outcome {
            AcceptedLegacyGapOutcome::Pass => assert_eq!(
                check_statement(&support::Stmt {
                    source: statement.clone(),
                }),
                Outcome::Pass,
                "{}:{} regressed from a resolved legacy gap",
                gap.file,
                gap.statement_index
            ),
            AcceptedLegacyGapOutcome::Diagnostic(expected) => {
                let failure = match pgsql_format(&statement) {
                    Ok(_) => panic!(
                        "{}:{} unexpectedly resolved; review the accepted-gap ledger",
                        gap.file, gap.statement_index
                    ),
                    Err(failure) => failure,
                };
                assert_eq!(
                    failure.diagnostic(),
                    Some(&StrictDiagnostic {
                        code: expected.code.clone(),
                        region: expected.region.clone(),
                        anchor: expected.anchor.clone(),
                    }),
                    "{}:{} diagnostic contract changed",
                    gap.file,
                    gap.statement_index
                );
            }
        }
    }
}
