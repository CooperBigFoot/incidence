#![allow(clippy::expect_used)]

mod support;

use incidence_core::disposition::{
    Allocation, Disposition, SubstanceDisposition, TransactionError, commit_disposition,
};
use incidence_core::ledger::{AuthoritativeLog, RunId};
use incidence_core::temporal::TimestepIndex;

#[test]
fn a_held_registered_substance_must_be_explicitly_disposed() {
    let artifact = support::fixture();
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([4; 16]), &artifact);
    let disposition = Disposition::new(
        TimestepIndex::new(1),
        support::finite(&artifact, "store"),
        [SubstanceDisposition::new(
            support::substance("water"),
            support::amount(8.0),
            [Allocation::new(
                support::endpoint(&artifact, "route"),
                support::amount(2.0),
            )],
        )],
    );

    let error = commit_disposition(&mut log, &artifact, disposition)
        .expect_err("salt omission must abort the transaction");

    assert_eq!(
        error,
        TransactionError::OmittedSubstance {
            compartment: support::compartment("store"),
            substance: support::substance("salt"),
            timestep: TimestepIndex::new(1),
        }
    );
    let diagnostic = error.to_string();
    assert!(diagnostic.contains("store"));
    assert!(diagnostic.contains("salt"));
    assert!(diagnostic.contains("1"));
    assert_eq!(log.transfer_count(), 0);
}

#[test]
fn complete_explicit_partitions_commit_their_outgoing_transfers() {
    let artifact = support::fixture();
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([7; 16]), &artifact);
    let disposition = Disposition::new(
        TimestepIndex::new(1),
        support::finite(&artifact, "store"),
        [
            SubstanceDisposition::new(
                support::substance("water"),
                support::amount(8.0),
                [Allocation::new(
                    support::endpoint(&artifact, "route"),
                    support::amount(2.0),
                )],
            ),
            SubstanceDisposition::new(support::substance("salt"), support::amount(4.0), []),
        ],
    );

    commit_disposition(&mut log, &artifact, disposition).expect("complete partition must commit");

    assert_eq!(log.transfer_count(), 1);
}
