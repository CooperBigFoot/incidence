//! End-to-end adequacy proof for the seven neutral hydrology fixtures.

#[path = "support/hydrology.rs"]
mod hydrology;

use std::collections::BTreeSet;

use incidence_core::execution::execute_model;
use incidence_core::ledger::{RunId, replay_with_artifact};

#[test]
fn all_seven_hydrology_rules_run_to_completion_in_one_model() {
    let artifact = hydrology::fixture()
        .artifact()
        .expect("public hydrology document");
    assert_eq!(artifact.rules().len(), hydrology::RULE_COMPARTMENTS.len());

    let log = execute_model(&artifact, RunId::from_bytes([0x71; 16]))
        .unwrap_or_else(|error| panic!("hydrology model failed: {error}"));
    assert!(log.is_sealed());
    assert_eq!(
        log.digest().to_hex(),
        "60c120b0d2a03ae4c9aada1d938a5d50be80398ac6d786cb2350c09b58e4ab67"
    );
    assert_eq!(
        artifact.digest().to_hex(),
        "e3330d288b2fbc90fad3e5f1eccb18331eeb1b6af235debadba4539dd10485fa"
    );
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
