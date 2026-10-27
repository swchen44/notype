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
    let free = entries
        .iter()
        .filter(|entry| {
            entry
                .api_summary
                .requested_models
                .iter()
                .any(|model| model == "google/gemma-4-31b-it:free")
        })
        .collect::<Vec<_>>();
    assert_eq!(free.len(), 3);
    for entry in free {
        assert_eq!(
            entry.api_summary.fees.status,
            notype_core::observations::Completeness::Complete
        );
        assert_eq!(
            entry.api_summary.fees.reported_calls,
            entry.api_summary.observed_call_count
        );
        assert_eq!(entry.api_summary.fees.totals.len(), 1);
        assert_eq!(entry.api_summary.fees.totals[0].currency, "USD");
        assert_eq!(entry.api_summary.fees.totals[0].amount, "0");
    }
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
    // Returned zero is an actual receipt, not a price estimate or an unknown.
    // Independent readback must reject a changed currency total in the CSV.
    let text = std::str::from_utf8(&csv).unwrap();
    let forged = text.replacen("\"\"amount\"\":\"\"0\"\"", "\"\"amount\"\":\"\"1\"\"", 1);
    assert_ne!(forged.as_bytes(), csv);
    assert!(probe::export::verify(forged.as_bytes(), &entries, &journal.snapshot()).is_err());
    assert!(root.join("results/diagnostics.txt").is_file());
}
