//! Executable round-trip proof for the hydrology fixture model document.

#[path = "support/hydrology.rs"]
mod hydrology;

use incidence_core::execution::execute_model;
use incidence_core::ledger::RunId;
use incidence_core::model_artifact::ModelArtifactArchive;

#[test]
fn decoded_hydrology_model_emits_the_identical_transfer_log() {
    let authored_document = hydrology::fixture();
    let encoded = serde_json::to_vec(&authored_document)
        .unwrap_or_else(|error| panic!("model serialization failed: {error}"));
    let decoded_document: incidence_core::model_document::ModelDocument =
        serde_json::from_slice(&encoded)
            .unwrap_or_else(|error| panic!("model deserialization failed: {error}"));
    let reencoded = serde_json::to_vec(&decoded_document)
        .unwrap_or_else(|error| panic!("model reserialization failed: {error}"));
    assert_eq!(reencoded, encoded);

    let authored = authored_document.artifact().expect("authored artifact");
    let decoded = decoded_document.artifact().expect("decoded artifact");
    assert_eq!(decoded.canonical_bytes(), authored.canonical_bytes());
    assert_eq!(decoded.digest(), authored.digest());

    let mut archive = ModelArtifactArchive::new();
    let digest = archive.insert(decoded).expect("archive insert");
    let retrieved = archive.get(&digest).expect("artifact by digest");
    let run_id = RunId::from_bytes([0x72; 16]);
    let authored_log = execute_model(&authored, run_id)
        .unwrap_or_else(|error| panic!("authored model failed: {error}"));
    let decoded_log = execute_model(&retrieved, run_id)
        .unwrap_or_else(|error| panic!("decoded model failed: {error}"));

    assert_eq!(decoded_log.digest(), authored_log.digest());
    assert_eq!(
        decoded_log.records().collect::<Vec<_>>(),
        authored_log.records().collect::<Vec<_>>(),
    );
}
