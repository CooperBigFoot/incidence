#[path = "support/execution.rs"]
mod support;
use incidence_core::execution::execute_model;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::ledger::{RunId, replay_with_artifact};
use incidence_core::presence::ValueState;
#[test]
fn executor_uses_forcing_and_commits_only_validated_transfers() {
    let artifact = support::artifact();
    let log = execute_model(&artifact, RunId::from_bytes([7; 16])).expect("run");
    assert!(log.is_sealed());
    assert_eq!(log.transfers().len(), 2);
    assert_eq!(
        log.transfers()[0]
            .amounts()
            .amount(&SubstanceId::parse("water").expect("id")),
        ValueState::Present(1.0.try_into().expect("amount"))
    );
    let replay = replay_with_artifact(&log, &artifact).expect("replay");
    assert_eq!(
        replay.final_state().finite_stock(
            &CompartmentId::parse("source").expect("id"),
            &SubstanceId::parse("water").expect("id")
        ),
        ValueState::Present(7.0.try_into().expect("amount"))
    );
}
