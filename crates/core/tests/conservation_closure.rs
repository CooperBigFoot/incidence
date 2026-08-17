#[path = "ledger.rs"]
mod criterion_suite;

use incidence_core::identity::SubstanceId;
use incidence_core::ledger::{
    AuthoritativeLog, RunId, Transfer, incidence_columns_close, replay_with_artifact,
};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::presence::ValueState;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::temporal::TimestepIndex;

#[test]
fn finite_and_boundary_accounts_close_exactly() {
    let artifact = criterion_suite::fixture(false, 10.0);
    let water = SubstanceId::parse("water").expect("valid substance");
    let amount = || {
        SparseSubstanceVector::new(
            artifact.registry(),
            [(
                water.clone(),
                NonNegativeAmount::try_from(3.0).expect("valid amount"),
            )],
        )
        .expect("valid vector")
    };
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([9; 16]), &artifact);
    log.append(Transfer::new(
        TimestepIndex::new(0),
        criterion_suite::endpoint(&artifact, "store"),
        criterion_suite::endpoint(&artifact, "route"),
        amount(),
    ))
    .expect("first transfer");
    log.append(Transfer::new(
        TimestepIndex::new(1),
        criterion_suite::endpoint(&artifact, "route"),
        criterion_suite::endpoint(&artifact, "outside_out"),
        amount(),
    ))
    .expect("second transfer");
    assert!(incidence_columns_close(&log));
    let replay = replay_with_artifact(&log, &artifact).expect("exact replay");
    let ValueState::Present(totals) = replay.conservation_totals(&water).expect("finite totals")
    else {
        panic!("modelled substance has totals");
    };
    assert!(totals.closes_bit_identically());
    assert_eq!(totals.genesis_total().to_bits(), 10.0_f64.to_bits());
}
