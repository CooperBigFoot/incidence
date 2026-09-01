#![allow(clippy::expect_used)]

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::execution_bindings::{
    ExecutionBindings, RuleInputBinding, RuleInputSource, TransferBranchBinding,
};
use incidence_core::forcing::ForcingSeries;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::model_artifact::{
    ModelArtifact, ModelArtifactError, Quantum, RuleDefinition, SubstanceUnit, UnitId,
};
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::partition_expression::PartitionExpr;
use incidence_core::projection::ProjectionSet;
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, TransferBranchId,
};
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{Topology, TopologyEndpoint};
use incidence_core::versions::RuleIrVersion;

type FixtureParts = (
    Topology,
    SubstanceRegistry,
    InitialStocks,
    FixedStepCalendar,
    RunHorizon,
    SubstanceId,
    CompartmentId,
    CompartmentId,
    CompartmentId,
);

fn parts() -> FixtureParts {
    let source = CompartmentId::parse("source").expect("valid id");
    let downstream = CompartmentId::parse("downstream").expect("valid id");
    let boundary = CompartmentId::parse("outside").expect("valid id");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(source.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(downstream.clone())),
            TopologyEndpoint::Boundary(BoundaryAccount::new(boundary.clone())),
        ],
        [],
    )
    .expect("valid topology");
    let water = SubstanceId::parse("water").expect("valid id");
    let registry = SubstanceRegistry::new([water.clone()]).expect("valid registry");
    let stocks = InitialStocks::new(&topology, &registry, []).expect("valid stocks");
    let horizon =
        RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(1)).expect("valid horizon");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("valid duration"),
    );
    (
        topology, registry, stocks, calendar, horizon, water, source, downstream, boundary,
    )
}

fn build(
    destination: CompartmentId,
    include_branch: bool,
) -> Result<ModelArtifact, ModelArtifactError> {
    let input = InputRef::new(
        InputId::parse("rain-input").expect("valid id"),
        ExpressionValueKind::Scalar,
    );
    build_with_input_binding(destination, include_branch, input.clone(), input)
}

fn build_with_input_binding(
    destination: CompartmentId,
    include_branch: bool,
    expression_input: InputRef,
    binding_input: InputRef,
) -> Result<ModelArtifact, ModelArtifactError> {
    let (topology, registry, stocks, calendar, horizon, water, source, _, _) = parts();
    let branch = TransferBranchId::parse("released").expect("valid id");
    let rain = ForcingId::parse("rain").expect("valid id");
    let rule = RuleDefinition::new(
        source.clone(),
        water.clone(),
        RuleExpr::input(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            expression_input,
        ),
        PartitionExpr::release_all(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            branch.clone(),
        ),
        [],
    )?;
    let transfers = if include_branch {
        vec![TransferBranchBinding::new(
            source.clone(),
            water.clone(),
            branch,
            destination,
        )]
    } else {
        vec![]
    };
    let bindings = ExecutionBindings::new(
        transfers,
        [RuleInputBinding::new(
            source,
            water.clone(),
            binding_input,
            RuleInputSource::Forcing(ForcingRef::new(rain.clone())),
        )],
    )
    .expect("bindings have distinct keys");
    ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("empty projections valid"))
        .with_forcings(vec![
            ForcingSeries::new(rain, horizon, vec![1.0, 2.0]).expect("valid forcing"),
        ])
        .with_rules(vec![rule])
        .with_execution_bindings(bindings)
        .with_units(vec![(
            water,
            SubstanceUnit::new(
                UnitId::parse("m3").expect("valid unit"),
                Quantum::try_from(1.0e-6).expect("valid quantum"),
            ),
        )])
        .build()
}

#[test]
fn branch_destinations_and_rule_input_sources_are_structurally_resolved() {
    let (_, _, _, _, _, water, source, downstream, boundary) = parts();
    let artifact = build(downstream.clone(), true).expect("all bindings resolve");
    let branch = TransferBranchId::parse("released").expect("valid id");
    let input = InputId::parse("rain-input").expect("valid id");
    assert_eq!(
        artifact.transfer_destination(&source, &water, &branch),
        Some(&downstream)
    );
    assert!(matches!(
        artifact.rule_input_source(&source, &water, &input),
        Some(RuleInputSource::Forcing(_))
    ));
    build(boundary, true).expect("a boundary account is a real destination endpoint");
}

#[test]
fn missing_and_undeclared_branch_destinations_never_produce_an_artifact() {
    let (_, _, _, _, _, water, source, _, _) = parts();
    let missing = build(source.clone(), false).expect_err("branch must be bound");
    assert!(
        matches!(missing, ModelArtifactError::MissingTransferBranchBinding { compartment, substance, branch }
        if compartment == source && substance == water && branch.as_str() == "released")
    );

    let nowhere = CompartmentId::parse("nowhere").expect("valid id");
    let unknown = build(nowhere.clone(), true).expect_err("destination must be declared");
    assert!(
        matches!(unknown, ModelArtifactError::UnknownTransferDestination { compartment, substance, branch, destination }
        if compartment == source && substance == water && branch.as_str() == "released" && destination == nowhere)
    );
}

#[test]
fn changing_only_a_branch_destination_changes_canonical_identity() {
    let (_, _, _, _, _, _, _, downstream, boundary) = parts();
    let first = build(downstream, true).expect("downstream binding resolves");
    let second = build(boundary, true).expect("boundary binding resolves");
    assert_ne!(first.canonical_bytes(), second.canonical_bytes());
    assert_ne!(first.digest(), second.digest());
}

#[test]
fn input_binding_cannot_misstate_the_rule_leaf_kind() {
    let (_, _, _, _, _, _, _, downstream, _) = parts();
    let input = InputId::parse("rain-input").expect("valid id");
    let result = build_with_input_binding(
        downstream,
        true,
        InputRef::new(input.clone(), ExpressionValueKind::Scalar),
        InputRef::new(input, ExpressionValueKind::Truth),
    );

    assert!(
        result.is_err(),
        "an artifact must reject binding metadata that disagrees with its rule leaf"
    );
}
