#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_root_activity.rs"]
mod activity;
use activity::ActivityGate;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_engine_state.rs"]
mod engine_state;
use engine_state::EngineState;

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

#[test]
fn a_late_engine_return_cannot_replace_a_new_load_of_the_same_artifact() {
    let mut state = EngineState::new();
    let first = state.begin_load("same-model-same-hash");
    assert!(state.publish(first, "old engine"));
    let old = state.take().unwrap();
    let next = state.begin_load("same-model-same-hash");
    assert!(state.publish(next, "new engine"));
    assert!(!state.return_engine(old));
    assert_eq!(state.as_ref(), Some(&"new engine"));
}

#[test]
fn a_late_panic_cannot_clear_a_new_load_of_the_same_artifact() {
    let mut state = EngineState::new();
    let first = state.begin_load("same artifact");
    state.publish(first, "old engine");
    let old = state.take().unwrap();
    let marker = old.marker().clone();
    let next = state.begin_load("same artifact");
    state.publish(next, "successor");
    assert!(!state.clear_if(&marker));
    assert_eq!(state.as_ref(), Some(&"successor"));
}

#[test]
fn manual_unload_invalidates_an_unfinished_native_load_and_allows_a_fresh_one() {
    let mut state = EngineState::new();
    let old = state.begin_load("same artifact");
    state.clear();
    assert!(!state.publish(old, "late result"));
    assert_eq!(state.current_key(), None);
    let fresh = state.begin_load("same artifact");
    assert!(state.publish(fresh, "fresh engine"));
    assert_eq!(state.as_ref(), Some(&"fresh engine"));
}

#[test]
fn an_actual_queued_worker_retains_admission_until_its_closure_finishes() {
    let gate = ActivityGate::new();
    let caller = gate.enter().unwrap();
    let worker = caller.worker();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let _actual_worker_owner = worker;
        entered_tx.send(()).unwrap();
        finish_rx.recv().unwrap();
    });
    entered_rx.recv().unwrap();
    drop(caller); // Cancellation acknowledgement is not a terminal worker.
    assert!(gate.transition().is_err());
    finish_tx.send(()).unwrap();
    handle.join().unwrap();
    assert!(gate.transition().is_ok());
}
