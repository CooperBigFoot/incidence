//! Falsifier for the linear-reservoir fixture's hydrologic recurrence.

#[path = "support/hydrology.rs"]
mod hydrology;

use incidence_core::execution::execute_model;
use incidence_core::ledger::RunId;

#[test]
fn linear_reservoir_releases_a_fraction_of_remaining_storage() {
    let artifact = hydrology::HydrologyModelDocument::fixture().artifact();
    let log = execute_model(&artifact, RunId::from_bytes([0x73; 16]))
        .unwrap_or_else(|error| panic!("hydrology model failed: {error}"));
    let releases = log
        .transfers()
        .iter()
        .filter(|transfer| transfer.source().id().as_str() == "linear-reservoir")
        .map(|transfer| {
            transfer
                .amounts()
                .iter()
                .next()
                .map(|(_, amount)| amount.value())
                .unwrap_or_else(|| panic!("linear-reservoir transfer has no substance amount"))
        })
        .collect::<Vec<_>>();

    assert_eq!(releases, vec![2.0, 1.5, 1.125]);
}
