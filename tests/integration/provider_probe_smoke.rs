//! The same synthetic production path used by the future Windows EXE check.
use super::*;
struct Space;
impl probe::bridge::ResourceGate for Space {
    fn checkpoint(&self) -> Result<(), probe::journal::Failure> {
        Ok(())
    }
}
impl notype_core::csv::CsvSpaceProbe for Space {
    fn available_bytes(
        &self,
        _: &std::path::Path,
    ) -> Result<u64, notype_core::csv::CsvExportError> {
        Ok(64 * 1024 * 1024)
    }
}
#[tokio::test]
async fn complete_same_helper_check_uses_owned_home_twelve_http_calls_nine_histories_and_real_csv()
{
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("smoke");
    let identity = probe::journal::Identity {
        source: "a".repeat(40),
        policy_sha256: "b".repeat(64),
        corpus_sha256: "c".repeat(64),
        contract_sha256: vec![],
    };
    let proof = probe::smoke::exercise(&root, identity, Arc::new(Space))
        .await
        .unwrap();
    assert_eq!(proof.accepted_requests, 12);
    assert_eq!(proof.observed_calls, 12);
    assert_eq!(proof.histories, 9);
    assert!(proof.gemini_sealed && proof.synthetic_keys_excluded);
    let journal = probe::journal::Journal::open_report(&root.join("round")).unwrap();
    assert!(journal.report_only());
    assert_eq!(journal.snapshot().reservation_units().unwrap(), 835_584);
    let core = SessionService::open(root.join("results/sessions.db")).unwrap();
    let entries = core.search("").unwrap();
    assert_eq!(entries.len(), 9);
    let retried = entries
        .iter()
        .find(|entry| {
            entry.api_summary.call_count == Some(2)
                && entry.transcript == probe::policy::CANCELLATION
        })
        .unwrap();
    assert_eq!(retried.api_summary.input.reported_calls, 1);
    assert_eq!(retried.api_summary.input.known_subtotal, Some(5));
    assert_eq!(retried.api_summary.fees.reported_calls, 0);
    let csv = std::fs::read(root.join("results/history.csv")).unwrap();
    assert_eq!(
        probe::export::verify(&csv, &entries, &journal.snapshot())
            .unwrap()
            .observed_calls,
        12
    );
    assert!(root.join("results/diagnostics.txt").is_file());
}
