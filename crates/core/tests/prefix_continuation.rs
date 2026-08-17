#[path = "support/execution.rs"]
mod support;
use incidence_core::execution::{ExecutionError, execute_model, resume_from_prefix};
use incidence_core::ledger::{AuthoritativeLog, Record, RunId};
#[test]
fn prefix_regenerates_disposable_state_and_emits_identical_suffix() {
    let artifact = support::artifact();
    let full = execute_model(&artifact, RunId::from_bytes([9; 16])).expect("run");
    let mut prefix = AuthoritativeLog::from_records([
        Record::Genesis(full.genesis().clone()),
        Record::Transfer(full.transfers()[0].clone()),
    ])
    .expect("prefix");
    resume_from_prefix(&artifact, &mut prefix).expect("resume");
    assert_eq!(
        prefix.records().collect::<Vec<_>>(),
        full.records().collect::<Vec<_>>()
    );
}

#[test]
fn completion_seal_is_not_a_resumable_prefix() {
    let artifact = support::artifact();
    let mut completed =
        execute_model(&artifact, RunId::from_bytes([9; 16])).expect("completed run");

    assert_eq!(
        resume_from_prefix(&artifact, &mut completed),
        Err(ExecutionError::AlreadyCompleted),
    );
}
