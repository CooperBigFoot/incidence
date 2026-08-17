#![allow(clippy::expect_used)]

use incidence_core::endpoints::FiniteCompartment;
use incidence_core::execution_bindings::{ExecutionBindings, TransferBranchBinding};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::model_artifact::{ModelArtifact, RuleDefinition, UnitId};
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::partition_expression::PartitionExpr;
use incidence_core::projection::ProjectionSet;
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::TransferBranchId;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{Topology, TopologyEndpoint};
use incidence_core::versions::RuleIrVersion;

#[test]
fn transfer_bindings_cannot_create_an_execution_cycle() {
    let first = CompartmentId::parse("first").expect("valid compartment");
    let second = CompartmentId::parse("second").expect("valid compartment");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(first.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(second.clone())),
        ],
        [],
    )
    .expect("declared topology is acyclic");
    let water = SubstanceId::parse("water").expect("valid substance");
    let registry = SubstanceRegistry::new([water.clone()]).expect("valid registry");
    let stocks = InitialStocks::new(&topology, &registry, []).expect("valid stocks");
    let horizon =
        RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(0)).expect("valid horizon");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("valid duration"),
    );
    let first_branch = TransferBranchId::parse("to-second").expect("valid branch");
    let second_branch = TransferBranchId::parse("to-first").expect("valid branch");
    let literal = || {
        RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, 1.0)
            .expect("finite literal")
    };
    let first_rule = RuleDefinition::new(
        first.clone(),
        water.clone(),
        literal(),
        PartitionExpr::release_all(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            first_branch.clone(),
        ),
        [],
    )
    .expect("valid first rule");
    let second_rule = RuleDefinition::new(
        second.clone(),
        water.clone(),
        literal(),
        PartitionExpr::release_all(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            second_branch.clone(),
        ),
        [],
    )
    .expect("valid second rule");
    let bindings = ExecutionBindings::new(
        [
            TransferBranchBinding::new(first.clone(), water.clone(), first_branch, second.clone()),
            TransferBranchBinding::new(second, water.clone(), second_branch, first),
        ],
        [],
    )
    .expect("binding coordinates are unique");

    let result = ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("valid projections"))
        .with_rules(vec![first_rule, second_rule])
        .with_execution_bindings(bindings)
        .with_units(vec![(water, UnitId::parse("m3").expect("valid unit"))])
        .build();

    assert!(
        result.is_err(),
        "bindings must not add a cycle hidden from the executor's topological traversal"
    );
}
