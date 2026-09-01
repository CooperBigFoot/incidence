//! Declared arithmetic quantum validation and model-identity proof.

#[path = "support/hydrology.rs"]
mod hydrology;

use incidence_core::model_artifact::MAX_EXACT_WHOLE_MULTIPLES;
use incidence_core::model_document::ModelDocument;

#[test]
fn quantum_is_required_in_a_unit_declaration() {
    let mut encoded = serde_json::to_value(hydrology::fixture()).expect("serialize fixture");
    encoded["units"][0]
        .as_object_mut()
        .expect("unit object")
        .remove("quantum");

    let error = serde_json::from_value::<ModelDocument>(encoded)
        .expect_err("a unit without a quantum must not decode");

    assert!(error.to_string().contains("units component"));
    assert!(error.to_string().contains("quantum"));
}

#[test]
fn uncountable_total_is_refused() {
    let mut document = hydrology::fixture();
    document.units[0].quantum = 100.0 / ((1_u64 << 51) as f64);

    let error = document
        .artifact()
        .expect_err("the initial total must fit the exactly countable range");
    let diagnostic = error.to_string();

    assert!(diagnostic.contains("water"));
    assert!(diagnostic.contains("declared total 700"), "{diagnostic}");
    assert!(diagnostic.contains("exactly countable ceiling"));
}

#[test]
fn quantum_participates_in_model_identity() {
    let mut coarse = hydrology::fixture();
    coarse.units[0].quantum = 1.0e-3;
    let mut fine = coarse.clone();
    fine.units[0].quantum = 1.0e-6;

    let coarse = coarse.artifact().expect("coarse quantum artifact");
    let fine = fine.artifact().expect("fine quantum artifact");

    assert_ne!(coarse.canonical_bytes(), fine.canonical_bytes());
    assert_ne!(coarse.digest(), fine.digest());
}

#[test]
fn split_initial_stocks_above_the_countable_ceiling_are_refused() {
    let mut document = hydrology::fixture();
    document.units[0].quantum = 1.0;
    for stock in &mut document.initial_stocks {
        stock.amounts[0].amount = 0.0;
    }
    document.initial_stocks[0].amounts[0].amount = MAX_EXACT_WHOLE_MULTIPLES;
    document.initial_stocks[1].amounts[0].amount = 1.0;

    let error = document
        .artifact()
        .expect_err("the extra quantum must not disappear while totaling stocks");
    let diagnostic = error.to_string();

    assert!(diagnostic.contains("water"), "{diagnostic}");
    assert!(
        diagnostic.contains("exactly countable ceiling"),
        "{diagnostic}"
    );
}

#[test]
fn misaligned_initial_stock_identifies_full_boundary_context() {
    let mut document = hydrology::fixture();
    document.units[0].quantum = 0.25;
    document.initial_stocks[0].amounts[0].amount = 1.1;

    let error = document
        .artifact()
        .expect_err("a fractional quantum must not enter conserved state");
    let diagnostic = error.to_string();

    assert!(diagnostic.contains("muskingum"), "{diagnostic}");
    assert!(diagnostic.contains("water"), "{diagnostic}");
    assert!(diagnostic.contains("1.1"), "{diagnostic}");
    assert!(diagnostic.contains("0.25"), "{diagnostic}");
}
