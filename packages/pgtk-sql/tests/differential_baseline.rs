//! Fast, oracle-free validation of the frozen differential baseline.

// This target uses only part of the shared baseline module.
#[allow(dead_code)]
#[path = "support/baseline.rs"]
mod baseline;

#[test]
fn accepted_legacy_gap_contract_has_one_identity_per_frozen_skip() {
    let gaps = baseline::AcceptedLegacyGaps::pinned();

    assert_eq!(gaps.entries().len(), 18);
    // The last diagnostic-outcome gap (join.sql:171, `USING t1 JOIN t2 USING`)
    // resolved when suffix-proving optional viability landed in Recursa.
    assert_eq!(
        gaps.entries()
            .iter()
            .filter(|gap| matches!(
                &gap.outcome,
                baseline::AcceptedLegacyGapOutcome::Diagnostic(_)
            ))
            .count(),
        0
    );
}

#[test]
fn every_version_baseline_is_complete_and_comes_from_its_pinned_oracle() {
    // Loading checks the corpus identity, the pin, the oracle outcome of
    // each frozen submission and the order and ranges of the expected
    // version gaps.
    for feature in baseline::version_features() {
        let version = baseline::VersionBaseline::of(feature);
        assert_eq!(version.feature, feature);
        assert_eq!(
            (version.oracle_ref.as_str(), version.oracle_commit.as_str()),
            baseline::oracle_pin(feature)
        );
    }
    assert_eq!(
        baseline::VersionBaseline::active().feature,
        pgtk_sql::TARGET_VERSION.feature()
    );
}

#[test]
fn the_frozen_submissions_cover_the_frozen_corpus() {
    let submissions = baseline::FrozenSubmissions::pinned();
    let legacy = baseline::FrozenStatements::pinned();
    assert_eq!(submissions.file_names(), legacy.file_names());
    for name in legacy.file_names() {
        let file = submissions.file(name);
        assert_eq!(file.source_git_blob, legacy.file(name).source_git_blob);
        assert_eq!(file.source_bytes, legacy.file(name).source_bytes);
    }
}

#[test]
fn every_version_checks_the_same_server_submissions() {
    // The statement set is the psql submissions, the same for each version:
    // the corpus is the 17.9 corpus, and psql 17 sends it.
    let pg17 = baseline::VersionBaseline::of("pg17");
    for feature in baseline::version_features() {
        let version = baseline::VersionBaseline::of(feature);
        assert_eq!(
            version.expected_totals().checked,
            pg17.expected_totals().checked,
            "{feature}"
        );
    }
}
