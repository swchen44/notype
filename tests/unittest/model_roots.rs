#[path = "../../desktop/src-tauri/src/managers/model_root_activity.rs"]
mod activity;
use activity::ActivityGate;

#[test]
fn cancelling_the_caller_cannot_apply_until_the_actual_worker_exits() {
    let gate = ActivityGate::new();
    let caller = gate.enter().unwrap();
    let worker = caller.clone();
    drop(caller);
    assert!(gate.transition().is_err());
    drop(worker);
    let apply = gate.transition().unwrap();
    assert!(gate.enter().is_err());
    drop(apply);
    assert!(gate.enter().is_ok());
}
