#![allow(clippy::expect_used)]

use incidence_core::endpoints::FiniteCompartment;
use incidence_core::forcing::ForcingSeries;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::model_artifact::{ModelArtifact, ModelArtifactArchive, ModelVersions, UnitId};
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::projection::ProjectionSet;
use incidence_core::rule_reference::ForcingId;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{Topology, TopologyEndpoint};
use incidence_core::versions::{CanonicalEncodingVersion, InterpreterVersion, RuleIrVersion};

fn fixture(value: f64, numerical: NumericalSemanticsVersion) -> ModelArtifact {
    let compartment = CompartmentId::parse("storage").expect("fixture identity is valid");
    let water = SubstanceId::parse("water").expect("fixture identity is valid");
    let topology = Topology::new(
        [TopologyEndpoint::Finite(FiniteCompartment::new(
            compartment,
        ))],
        [],
    )
    .expect("fixture topology is valid");
    let registry = SubstanceRegistry::new([water.clone()]).expect("fixture registry is valid");
    let stocks = InitialStocks::new(&topology, &registry, [])
        .expect("empty initial stocks are explicit zero");
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(1))
        .expect("fixture horizon is valid");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(86_400).expect("fixture duration is valid"),
    );
    let forcing = ForcingSeries::new(
        ForcingId::parse("rain").expect("fixture identity is valid"),
        horizon,
        vec![1.0, value],
    )
    .expect("fixture forcing is valid");
    ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(
            ProjectionSet::new(vec![], vec![]).expect("empty projection set is valid"),
        )
        .with_forcings(vec![forcing])
        .with_units(vec![(
            water,
            UnitId::parse("m3").expect("fixture unit is valid"),
        )])
        .with_versions(ModelVersions::new(
            RuleIrVersion::V1,
            InterpreterVersion::V1,
            numerical,
            CanonicalEncodingVersion::V1,
        ))
        .build()
        .expect("fixture artifact is valid")
}

#[test]
fn digest_discriminates_forcing_values_and_numerical_semantics_versions() {
    let base = fixture(2.0, NumericalSemanticsVersion::V1);
    let changed_forcing = fixture(2.5, NumericalSemanticsVersion::V1);
    let changed_numerical_version = fixture(2.0, NumericalSemanticsVersion::V2);

    assert_ne!(base.digest(), changed_forcing.digest());
    assert_ne!(base.digest(), changed_numerical_version.digest());
    assert_ne!(base.canonical_bytes(), changed_forcing.canonical_bytes());
    assert_eq!(base.digest().as_bytes().len(), 32);
    assert_eq!(base.digest().to_string().len(), 64);
}

#[test]
fn archive_retains_every_historical_artifact_by_digest() {
    let first = fixture(2.0, NumericalSemanticsVersion::V1);
    let second = fixture(2.5, NumericalSemanticsVersion::V1);
    let first_digest = first.digest();
    let second_digest = second.digest();
    let mut archive = ModelArtifactArchive::new();

    archive.insert(first).expect("first artifact inserts");
    archive.insert(second).expect("second artifact inserts");

    assert_eq!(archive.len(), 2);
    assert_eq!(
        archive
            .get(&first_digest)
            .expect("first remains retrievable")
            .digest(),
        first_digest
    );
    assert_eq!(
        archive
            .get(&second_digest)
            .expect("second remains retrievable")
            .digest(),
        second_digest
    );
}
