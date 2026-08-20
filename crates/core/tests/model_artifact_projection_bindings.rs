#![allow(clippy::expect_used)]

use incidence_core::endpoints::FiniteCompartment;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::model_artifact::{ModelArtifact, UnitId};
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::projection::{
    AuthoritativeFactSelector, BoundedLagSpec, InitialProjectionValue, InitialProjectorState,
    ProjectionSet, ProjectionSource,
};
use incidence_core::rule_reference::ProjectionId;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{Topology, TopologyEndpoint};
use incidence_core::versions::RuleIrVersion;

#[test]
fn rejects_projection_fact_selectors_outside_artifact_domains() {
    let storage = CompartmentId::parse("storage").expect("fixture identity is valid");
    let missing = CompartmentId::parse("missing").expect("fixture identity is valid");
    let water = SubstanceId::parse("water").expect("fixture identity is valid");
    let topology = Topology::new(
        [TopologyEndpoint::Finite(FiniteCompartment::new(storage))],
        [],
    )
    .expect("fixture topology is valid");
    let registry = SubstanceRegistry::new([water.clone()]).expect("fixture registry is valid");
    let stocks = InitialStocks::new(&topology, &registry, [])
        .expect("empty initial stocks are explicit zero");
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(0))
        .expect("fixture horizon is valid");
    let projection_id = ProjectionId::parse("missing-inflow").expect("fixture identity is valid");
    let projection = BoundedLagSpec::new(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        projection_id.clone(),
        ProjectionSource::AuthoritativeFact(AuthoritativeFactSelector::IncomingTransferAmount {
            compartment: missing,
            substance: water.clone(),
        }),
        1,
    )
    .expect("projection is internally valid");
    let state = InitialProjectorState::new(
        projection_id,
        NumericalSemanticsVersion::V1,
        vec![InitialProjectionValue::Extensive(0.0)],
    )
    .expect("projection state is valid");
    let projections = ProjectionSet::new(vec![projection.into()], vec![state])
        .expect("projection set is internally valid");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(86_400).expect("fixture duration is valid"),
    );

    let error = ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(projections)
        .with_units(vec![(
            water,
            UnitId::parse("m3").expect("fixture unit is valid"),
        )])
        .build()
        .expect_err("a projection cannot read an absent compartment");

    assert_eq!(
        error.to_string(),
        "projection `missing-inflow` refers to compartment `missing`, which is absent from the topology"
    );
}
