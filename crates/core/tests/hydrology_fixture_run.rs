//! End-to-end adequacy proof for the seven neutral hydrology fixtures.

#[path = "support/hydrology.rs"]
mod hydrology;

use std::collections::BTreeSet;

use incidence_core::execution::execute_model;
use incidence_core::ledger::{RunId, replay_with_artifact};

#[test]
fn all_seven_hydrology_rules_run_to_completion_in_one_model() {
    let artifact = hydrology::HydrologyModelDocument::fixture().artifact();
    assert_eq!(artifact.rules().len(), hydrology::RULE_COMPARTMENTS.len());

    let log = execute_model(&artifact, RunId::from_bytes([0x71; 16]))
        .unwrap_or_else(|error| panic!("hydrology model failed: {error}"));
    assert!(log.is_sealed());
    replay_with_artifact(&log, &artifact)
        .unwrap_or_else(|error| panic!("completed fixture did not replay: {error}"));

    let emitters = log
        .transfers()
        .iter()
        .map(|transfer| transfer.source().id().as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        emitters,
        hydrology::RULE_COMPARTMENTS.into_iter().collect(),
        "every fixture rule must be load-bearing in the completed run",
    );
}
