#![allow(clippy::expect_used)]
//! End-to-end power operation tests for the rule IR, model document, and interpreter.

#[path = "support/hydrology.rs"]
mod hydrology;

use incidence_core::execution::execute_model;
use incidence_core::ledger::{RunId, TransferEndpoint};
use incidence_core::model_document::ModelDocument;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::presence::ValueState;
use incidence_core::rule_expression::RuleExpr;
use incidence_core::temporal::TimestepIndex;
use incidence_core::versions::RuleIrVersion;

fn literal(value: f64) -> RuleExpr {
    RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value)
        .expect("finite power fixture literal")
}

#[test]
fn power_operation_roundtrips_through_the_model_document() {
    let expression = RuleExpr::power(literal(6.25), literal(0.5)).expect("power expression");
    let mut document = hydrology::fixture();
    document.rules[0].expression = expression.clone();

    let encoded = serde_json::to_vec(&document).expect("encode model document");
    let decoded: ModelDocument = serde_json::from_slice(&encoded).expect("decode model document");

    assert_eq!(decoded.rules[0].expression, expression);
    assert_eq!(decoded, document);
    assert_eq!(
        serde_json::to_value(&decoded.rules[0].expression).expect("encode expression"),
        serde_json::json!({
            "rule_ir_version": "v1",
            "numerical_semantics_version": "v1",
            "expression": {
                "kind": "power",
                "lhs": {"kind": "literal", "value": 6.25},
                "rhs": {"kind": "literal", "value": 0.5}
            }
        })
    );
}

#[test]
fn power_operation_computes_exactly_through_the_interpreter() {
    let mut document = hydrology::fixture();
    document.rules[0].expression =
        RuleExpr::power(literal(6.25), literal(0.5)).expect("power expression");
    let artifact = document.artifact().expect("compile model document");

    let log = execute_model(&artifact, RunId::from_bytes([0x50; 16])).expect("execute model");
    let transfer = log
        .transfers()
        .iter()
        .find(|transfer| {
            transfer.timestep() == TimestepIndex::new(0)
                && matches!(
                    transfer.source(),
                    TransferEndpoint::Finite(source) if source.id().as_str() == "muskingum"
                )
        })
        .expect("muskingum transfer at timestep zero");

    let water = incidence_core::identity::SubstanceId::parse("water").expect("water identity");
    match transfer.amounts().amount(&water) {
        ValueState::Present(amount) => assert_eq!(amount.value(), 2.5),
        state => panic!("power transfer must be present, got {state:?}"),
    }
}

#[test]
fn power_operation_preserves_v2_numerical_semantics_when_roundtripped() {
    let literal = |value| {
        RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V2, value)
            .expect("finite V2 power fixture literal")
    };
    let expression = RuleExpr::power(literal(6.25), literal(0.5)).expect("V2 power expression");

    let encoded = serde_json::to_value(&expression).expect("encode V2 power expression");
    assert_eq!(encoded["numerical_semantics_version"], "v2");

    let decoded: RuleExpr = serde_json::from_value(encoded).expect("decode V2 power expression");
    assert_eq!(decoded, expression);
}
