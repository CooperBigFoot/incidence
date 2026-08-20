#![allow(clippy::expect_used)]

use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::model_artifact::{RuleParameterSubstitution, RuleParameterSubstitutionError};
use incidence_core::model_document::ModelDocument;
use incidence_core::rule_reference::ParameterId;

fn substitution(parameter: &str, value: f64) -> RuleParameterSubstitution {
    RuleParameterSubstitution::new(
        CompartmentId::parse("linear-reservoir").expect("compartment"),
        SubstanceId::parse("water").expect("substance"),
        ParameterId::parse(parameter).expect("parameter"),
        value,
    )
}

fn artifact() -> incidence_core::model_artifact::ModelArtifact {
    serde_json::from_str::<ModelDocument>(include_str!(
        "../../../bindings/python/tests/fixture.json"
    ))
    .expect("fixture document")
    .into_artifact()
    .expect("fixture artifact")
}

#[test]
fn substitution_is_atomic_and_changes_the_derived_identity_only() {
    let held = artifact();
    let held_digest = held.digest();

    let first = held
        .with_rule_parameter_substitutions([substitution("linear-coefficient", 0.3)])
        .expect("declared parameter substitutes");
    let second = held
        .with_rule_parameter_substitutions([substitution("linear-coefficient", 0.4)])
        .expect("declared parameter substitutes");

    assert_eq!(held.digest(), held_digest);
    assert_ne!(first.digest(), held_digest);
    assert_ne!(first.digest(), second.digest());
}

#[test]
fn an_undeclared_parameter_names_the_refused_coordinate() {
    let held = artifact();
    let held_digest = held.digest();

    let error = held
        .with_rule_parameter_substitutions([substitution("forcing-rain", 0.3)])
        .expect_err("undeclared parameter must fail");

    assert!(matches!(
        error,
        RuleParameterSubstitutionError::ParameterNotDeclared { .. }
    ));
    assert!(error.to_string().contains("linear-reservoir"));
    assert!(error.to_string().contains("forcing-rain"));
    assert!(error.to_string().contains("not substitutable"));
    assert_eq!(held.digest(), held_digest);
}
