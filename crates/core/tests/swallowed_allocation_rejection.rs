#![allow(clippy::expect_used)]

mod support;

use incidence_core::disposition::{
    Allocation, Disposition, SubstanceDisposition, commit_disposition,
};
use incidence_core::ledger::{AuthoritativeLog, RunId};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::temporal::TimestepIndex;

#[test]
fn a_positive_allocation_that_cannot_debit_its_source_is_rejected_atomically() {
    let artifact = support::fixture();
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([8; 16]), &artifact);
    let before_digest = log.digest();
    let smallest_positive =
        NonNegativeAmount::try_from(f64::from_bits(1)).expect("finite positive amount");
    let disposition = Disposition::new(
        TimestepIndex::new(1),
        support::finite(&artifact, "store"),
        [
            SubstanceDisposition::new(
                support::substance("water"),
                support::amount(10.0),
                [Allocation::new(
                    support::endpoint(&artifact, "route"),
                    smallest_positive,
                )],
            ),
            SubstanceDisposition::new(support::substance("salt"), support::amount(4.0), []),
        ],
    );

    let error = commit_disposition(&mut log, &artifact, disposition)
        .expect_err("a positive transfer must produce an exact source debit");

    let diagnostic = error.to_string();
    assert!(diagnostic.contains("store"));
    assert!(diagnostic.contains("water"));
    assert!(diagnostic.contains("1"));
    assert_eq!(log.transfer_count(), 0);
    assert_eq!(log.digest(), before_digest);
}
