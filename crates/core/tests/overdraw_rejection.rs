#![allow(clippy::expect_used)]

mod support;

use incidence_core::disposition::{
    Allocation, Disposition, SubstanceDisposition, TransactionError, commit_disposition,
};
use incidence_core::ledger::{AuthoritativeLog, RunId, replay_with_artifact};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::presence::ValueState;
use incidence_core::temporal::TimestepIndex;

#[test]
fn aggregate_overdraw_rejects_the_whole_transaction() {
    let artifact = support::fixture();
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([5; 16]), &artifact);
    let before_digest = log.digest();
    let disposition = Disposition::new(
        TimestepIndex::new(1),
        support::finite(&artifact, "store"),
        [
            SubstanceDisposition::new(
                support::substance("water"),
                support::amount(0.0),
                [
                    Allocation::new(support::endpoint(&artifact, "route"), support::amount(7.0)),
                    Allocation::new(
                        support::endpoint(&artifact, "outside"),
                        support::amount(5.0),
                    ),
                ],
            ),
            SubstanceDisposition::new(support::substance("salt"), support::amount(4.0), []),
        ],
    );

    let error = commit_disposition(&mut log, &artifact, disposition)
        .expect_err("aggregate overdraw must abort the transaction");

    assert!(matches!(
        error,
        TransactionError::Overdraw {
            ref compartment,
            ref substance,
            timestep,
            ..
        } if *compartment == support::compartment("store")
            && *substance == support::substance("water")
            && timestep == TimestepIndex::new(1)
    ));
    let diagnostic = error.to_string();
    assert!(diagnostic.contains("store"));
    assert!(diagnostic.contains("water"));
    assert!(diagnostic.contains("1"));
    assert_eq!(log.transfer_count(), 0);
    assert_eq!(log.digest(), before_digest);
    let replay = replay_with_artifact(&log, &artifact).expect("unchanged Genesis replays");
    assert_eq!(
        replay
            .final_state()
            .finite_stock(&support::compartment("store"), &support::substance("water")),
        ValueState::Present(NonNegativeAmount::try_from(10.0).expect("valid amount"))
    );
}

#[test]
fn a_late_invalid_transfer_leaves_earlier_staged_transfers_unpublished() {
    let artifact = support::fixture();
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([6; 16]), &artifact);
    let disposition = Disposition::new(
        TimestepIndex::new(1),
        support::finite(&artifact, "store"),
        [
            SubstanceDisposition::new(
                support::substance("water"),
                support::amount(0.0),
                [
                    Allocation::new(support::endpoint(&artifact, "route"), support::amount(2.0)),
                    Allocation::new(support::endpoint(&artifact, "store"), support::amount(8.0)),
                ],
            ),
            SubstanceDisposition::new(support::substance("salt"), support::amount(4.0), []),
        ],
    );

    let error = commit_disposition(&mut log, &artifact, disposition)
        .expect_err("invalid later transfer must reject the staged transaction");

    assert!(matches!(error, TransactionError::Replay { .. }));
    assert_eq!(log.transfer_count(), 0);
}
