//! Fixed headless modes. Arguments are rejected before any path/key/network
//! access. Only this module can reach the live selected-key resolution path.
use super::{
    bridge::ResourceGate,
    configuration::LiveConfigurations,
    evidence::FreshContracts,
    journal::{self, Failure, Identity, Journal, Snapshot, State},
    native::{self, Verified},
    runner::{PreparedRoot, ProbeRunner},
    smoke::{self, Trace},
};
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, path::Path, sync::Arc};
/// Three literal modes; no provider/model/root/budget/credential override.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Same downloaded binary, full synthetic path, no real home/provider.
    Check,
    /// One fresh gated fixed round; retained state is report-only.
    Run,
    /// Read retained safe evidence only, without refresh/export/key lookup.
    Report,
}
impl Mode {
    /// Parse only arguments after the program name. Never format rejected text.
    pub fn parse(arguments: &[OsString]) -> Result<Self, &'static str> {
        if arguments.len() != 1 {
            return Err("probe-arguments-unavailable");
        }
        match arguments[0].to_str() {
            Some("check") => Ok(Self::Check),
            Some("run") => Ok(Self::Run),
            Some("report") => Ok(Self::Report),
            _ => Err("probe-arguments-unavailable"),
        }
    }
}
/// Closed terminal labels; neither completion nor cancellation proves a bill.
#[derive(Clone, Copy, Serialize)]
pub enum ExitState {
    /// No selected key or inference admission when prerequisites fail.
    NotReady,
    /// Retained/partial identity cannot be reset, rearmed or resumed.
    ReportOnly,
    /// All eligible calls reconciled with four permanent Gemini skips.
    CompletedAutomated,
    /// Remaining calls stopped; prior reservations/evidence are retained.
    Halted,
}
/// Safe stdout/receipt projection. Raw results live only in owned production
/// SQLite/CSV; no credential/body/error/path is accepted by this type.
#[derive(Serialize)]
pub struct Summary {
    /// Distinguishes the no-key synthetic check from retained/live observations.
    pub synthetic: bool,
    /// Truthful local terminal lifecycle, independent of diagnostics preference.
    pub state: ExitState,
    /// Exact retained source, not a server/model attestation.
    pub source: String,
    /// Independently reconciled production calls, excluding pending reservations.
    pub reconciled_calls: usize,
    /// Conservative reserved slots; possibly sent is not confirmed server receipt.
    pub reserved_slots: usize,
    /// Retained USD1e-7 authorization, never actual billed currency.
    pub reservation_units: u64,
    /// Only successful sealed OpenRouter calls prove numeric reported zero.
    pub reported_zero_validated: usize,
    /// Actual production billed fees were never inferred or rewritten.
    pub actual_fee: &'static str,
    /// Requested/reported strings remain separate; no invented alias equality.
    pub identity_limitation: &'static str,
}
fn summary(snapshot: &Snapshot, state: ExitState) -> Result<Summary, Failure> {
    Ok(Summary {
        synthetic: false,
        state,
        source: snapshot.identity.source.clone(),
        reconciled_calls: snapshot
            .records
            .iter()
            .filter(|record| record.state == State::Reconciled)
            .count(),
        reserved_slots: snapshot
            .records
            .iter()
            .filter(|record| {
                matches!(
                    record.state,
                    State::PossiblySent | State::Finished | State::Reconciled
                )
            })
            .count(),
        reservation_units: snapshot.reservation_units()?,
        reported_zero_validated: snapshot
            .records
            .iter()
            .filter(|record| record.reported_zero_validated)
            .count(),
        actual_fee: "UNKNOWN",
        identity_limitation: "reported identity only; equivalence unverified",
    })
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckReceipt {
    source: String,
    binary_sha256: String,
    passed: bool,
    d_runtime_allocation_hwm: u64,
    proof: smoke::SmokeProof,
}
#[derive(Serialize)]
struct ProgressReceipt {
    last_slot: u8,
    authorization: Snapshot,
    actual_fee: &'static str,
    d_runtime_allocation_hwm: u64,
}
// Automatic reverse stays inside SessionService, so its actual observer begin
// must see the same freshness check as explicit outer steps. No token can
// attest to or extend the immutable monotonic evidence window.
struct LiveResources {
    quota: Arc<native::QuotaGate>,
    contracts: Arc<FreshContracts>,
}
impl ResourceGate for LiveResources {
    fn checkpoint(&self) -> Result<(), Failure> {
        self.contracts.require_fresh()?;
        self.quota.checkpoint()
    }
}
fn parent(root: &Path) -> Result<(), Failure> {
    journal::plain(root)?;
    let parent = root.parent().ok_or(Failure::Storage)?;
    if !parent.exists() {
        std::fs::create_dir(parent).map_err(|_| Failure::Storage)?;
    }
    journal::plain(parent)
}
fn absent(root: &Path) -> Result<(), Failure> {
    journal::plain(root)?;
    if root.exists() {
        Err(Failure::ReportOnly)
    } else {
        Ok(())
    }
}
fn synthetic_identity() -> Result<Identity, Failure> {
    Ok(Identity {
        source: native::source()?.into(),
        policy_sha256: native::sha256(include_bytes!("policy.rs"))?,
        corpus_sha256: native::sha256(include_bytes!("smoke.rs"))?,
        contract_sha256: vec![],
    })
}
fn check_receipt(verified: &Verified) -> Result<CheckReceipt, Failure> {
    let receipt: CheckReceipt = serde_json::from_slice(&native::read(
        &verified.artifact.join("check.json"),
        16_384,
    )?)
    .map_err(|_| Failure::NotReady)?;
    if !receipt.passed
        || receipt.source != native::source()?
        || receipt.binary_sha256 != verified.readiness.binary_sha256
        || receipt.proof.accepted_requests != 12
        || receipt.proof.observed_calls != 12
        || receipt.proof.histories != 9
        || !receipt.proof.gemini_sealed
        || !receipt.proof.synthetic_keys_excluded
    {
        return Err(Failure::NotReady);
    }
    Ok(receipt)
}
async fn check() -> Result<Summary, Failure> {
    let live = native::live_root();
    absent(&live)?;
    let verified = Verified::load()?;
    let root = native::smoke_root();
    absent(&root)?;
    let resources = verified.resources(root.clone());
    resources.checkpoint()?;
    parent(&root)?;
    let proof = smoke::exercise(&root, synthetic_identity()?, resources.clone()).await?;
    absent(&live)?;
    resources.checkpoint()?;
    let receipt = CheckReceipt {
        source: native::source()?.into(),
        binary_sha256: verified.readiness.binary_sha256.clone(),
        passed: true,
        d_runtime_allocation_hwm: resources.high_water()?,
        proof,
    };
    if verified.readiness.c_allocation_hwm.saturating_add(65_536) > 112 * 1_048_576 {
        return Err(Failure::Resource);
    }
    native::receipt(&verified.artifact.join("check.json"), &receipt)?;
    let journal = Journal::open_report(&root.join("round"))?;
    let mut summary = summary(&journal.snapshot(), ExitState::CompletedAutomated)?;
    summary.synthetic = true;
    Ok(summary)
}
fn report() -> Result<Summary, Failure> {
    let root = native::live_root();
    journal::plain(&root)?;
    native::validate_runtime(&root)?;
    if !root.is_dir() {
        return Err(Failure::ReportOnly);
    }
    let round = root.join("round");
    if !round.exists() {
        if native::read(&root.join("preparation.lock"), 128)?
            != b"notype-owned-P25-live-v4-preparation\n"
        {
            return Err(Failure::Storage);
        }
        return Ok(Summary {
            synthetic: false,
            state: ExitState::ReportOnly,
            source: String::new(),
            reconciled_calls: 0,
            reserved_slots: 0,
            reservation_units: 0,
            reported_zero_validated: 0,
            actual_fee: "UNKNOWN",
            identity_limitation: "unarmed partial preparation; no resume",
        });
    }
    let journal = Journal::open_report(&round)?;
    summary(&journal.snapshot(), ExitState::ReportOnly)
}
async fn run() -> Result<Summary, Failure> {
    let root = native::live_root();
    journal::plain(&root)?;
    if root.exists() {
        return report();
    }
    let verified = Verified::load()?;
    let check = check_receipt(&verified)?;
    let resources = verified.resources(root.clone());
    resources.retain_high_water(check.d_runtime_allocation_hwm)?;
    resources.checkpoint()?;
    parent(&root)?;
    // One atomic fresh directory permanently consumes this identity before a
    // public GET or selected key lookup; a failed preparation cannot reset it.
    let prepared = PreparedRoot::create(&root)?;
    let contracts = Arc::new(FreshContracts::acquire().await?);
    native::receipt(&root.join("contracts.json"), contracts.facts())?;
    resources.checkpoint()?;
    contracts.require_fresh()?;
    let identity = contracts.identity()?;
    // Lazy production native-home adapter; this line is unreachable before all
    // source/artifact/same-binary check/fresh-evidence/resource gates above.
    let configurations =
        LiveConfigurations::resolve(&crate::provider_config::credentials::CurrentUserReader)?;
    let trace = Arc::new(Trace::new());
    let mut runner = ProbeRunner::live(
        prepared,
        configurations,
        identity,
        trace.context(),
        Arc::new(LiveResources {
            quota: resources.clone(),
            contracts: contracts.clone(),
        }),
    )?;
    let processing = async {
        loop {
            contracts.require_fresh()?;
            resources.checkpoint()?;
            let Some(step) = runner.next().await? else {
                break;
            };
            runner.export(&root.join("results/history.csv"), resources.as_ref())?;
            resources.checkpoint()?;
            trace.export(&root.join(format!("results/diagnostics-{:02}.txt", step.last_slot)))?;
            let state = runner.snapshot()?;
            native::receipt(
                &root.join(format!("results/receipt-{:02}.json", step.last_slot)),
                &ProgressReceipt {
                    last_slot: step.last_slot,
                    authorization: state,
                    actual_fee: "UNKNOWN",
                    d_runtime_allocation_hwm: resources.high_water()?,
                },
            )?;
        }
        Ok::<(), Failure>(())
    }
    .await;
    if processing.is_err() {
        runner.halt()?;
    }
    let snapshot = runner.snapshot()?;
    let completed = processing.is_ok()
        && !snapshot.halted
        && snapshot.records[..12]
            .iter()
            .all(|record| record.state == State::Reconciled)
        && snapshot.records[12..]
            .iter()
            .all(|record| record.state == State::SealedGemini);
    let result = summary(
        &snapshot,
        if completed {
            ExitState::CompletedAutomated
        } else {
            ExitState::Halted
        },
    )?;
    resources.checkpoint()?;
    native::receipt(&root.join("results/final.json"), &result)?;
    Ok(result)
}
/// Execute an already parsed literal mode. Windows artifact verification fails
/// closed on other platforms before a home lookup or inference can be reached.
pub async fn execute(mode: Mode) -> Result<Summary, Failure> {
    match mode {
        Mode::Check => check().await,
        Mode::Run => run().await,
        Mode::Report => report(),
    }
}
