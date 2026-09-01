//! Public whole-model document equivalence and stable transport proof.

#[path = "support/hydrology.rs"]
mod hydrology;

use incidence_core::model_document::ModelDocument;

#[test]
fn document_decodes_to_the_same_canonical_artifact() {
    let document = hydrology::fixture();
    let encoded = serde_json::to_vec(&document).expect("serialize model document");
    let decoded: ModelDocument = serde_json::from_slice(&encoded).expect("decode model document");
    assert_eq!(
        serde_json::to_vec(&decoded).expect("reserialize model document"),
        encoded
    );

    let authored = document.artifact().expect("compile authored document");
    let reparsed = decoded.artifact().expect("compile decoded document");
    assert_eq!(authored.canonical_bytes(), reparsed.canonical_bytes());
    assert_eq!(authored.digest(), reparsed.digest());
    assert_eq!(
        authored.digest().to_hex(),
        "e3330d288b2fbc90fad3e5f1eccb18331eeb1b6af235debadba4539dd10485fa"
    );
}

#[test]
fn author_order_does_not_change_artifact_identity() {
    let original = hydrology::fixture();
    let mut reordered = original.clone();
    reordered.finite_compartments.reverse();
    reordered.connections.reverse();
    reordered.substances.reverse();
    reordered.initial_stocks.reverse();
    reordered.forcings.reverse();
    reordered.interpolation_tables.reverse();
    reordered.rules.reverse();
    reordered.transfer_bindings.reverse();
    reordered.input_bindings.reverse();
    reordered.units.reverse();
    assert_eq!(
        original.artifact().expect("original").digest(),
        reordered.artifact().expect("reordered").digest()
    );
}

#[test]
fn invalid_initial_stock_is_rejected_without_an_artifact() {
    let mut document = hydrology::fixture();
    document.initial_stocks[0].amounts[0].amount = -1.0;
    let error = document.artifact().expect_err("negative stock must fail");
    assert!(
        error
            .to_string()
            .contains("non-negative amount cannot be negative")
    );
}

#[test]
fn contradictory_projection_value_kind_is_rejected() {
    let mut encoded = serde_json::to_value(hydrology::fixture()).expect("serialize document");
    let specifications = encoded["projections"]["specifications"]
        .as_array_mut()
        .expect("projection specifications");
    assert_eq!(specifications[0]["value_kind"], "extensive");
    specifications[0]["value_kind"] = serde_json::json!("truth");

    let error = serde_json::from_value::<ModelDocument>(encoded)
        .expect_err("a projection kind may not contradict its specification");
    assert!(
        error
            .to_string()
            .contains("declared value kind Truth does not match derived Extensive")
    );
}

#[test]
fn unknown_document_version_is_rejected_during_decode() {
    let encoded = serde_json::to_string(&hydrology::fixture()).expect("serialize model document");
    let unknown = encoded.replacen(
        r#""document_version":"v1""#,
        r#""document_version":"v2""#,
        1,
    );
    let error =
        serde_json::from_str::<ModelDocument>(&unknown).expect_err("unknown version must fail");
    assert!(error.to_string().contains("unknown variant `v2`"));
}
