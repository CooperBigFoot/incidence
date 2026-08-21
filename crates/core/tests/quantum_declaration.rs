//! Declared arithmetic quantum validation and model-identity proof.

#[path = "support/hydrology.rs"]
mod hydrology;

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
    document.units[0].quantum = 1.0e-15;

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
