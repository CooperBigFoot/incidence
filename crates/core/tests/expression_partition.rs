#![allow(clippy::expect_used)]

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::execution::{ExecutionError, execute_model};
use incidence_core::execution_bindings::{ExecutionBindings, TransferBranchBinding};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::ledger::{RunId, replay_with_artifact};
use incidence_core::model_artifact::{ModelArtifact, RuleDefinition, UnitId};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::partition_expression::{ExpressionBranch, PartitionExpr};
use incidence_core::presence::ValueState;
use incidence_core::projection::ProjectionSet;
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::TransferBranchId;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};
use incidence_core::versions::RuleIrVersion;

fn id(value: &str) -> CompartmentId {
    CompartmentId::parse(value).expect("valid compartment")
}

fn branch(value: &str) -> TransferBranchId {
    TransferBranchId::parse(value).expect("valid branch")
}

fn literal(value: f64) -> RuleExpr {
    RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value)
        .expect("valid literal")
}

fn artifact(amounts: [f64; 2]) -> ModelArtifact {
    let source = id("source");
    let sink_a = id("sink-a");
    let sink_b = id("sink-b");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(source.clone())),
            TopologyEndpoint::Boundary(BoundaryAccount::new(sink_a.clone())),
            TopologyEndpoint::Boundary(BoundaryAccount::new(sink_b.clone())),
        ],
        [
            DirectedConnection::new(source.clone(), sink_a.clone()),
            DirectedConnection::new(source.clone(), sink_b.clone()),
        ],
    )
    .expect("valid topology");
    let water = SubstanceId::parse("water").expect("valid substance");
    let registry = SubstanceRegistry::new([water.clone()]).expect("valid registry");
    let stocks = InitialStocks::new(
        &topology,
        &registry,
        [(
            source.clone(),
            SparseSubstanceVector::new(
                &registry,
                [(
                    water.clone(),
                    NonNegativeAmount::try_from(10.0).expect("valid amount"),
                )],
            )
            .expect("valid stock vector"),
        )],
    )
    .expect("valid stocks");
    let horizon =
        RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(0)).expect("valid horizon");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("valid duration"),
    );
    let to_a = branch("to-a");
    let to_b = branch("to-b");
    let disposition = PartitionExpr::expression_partition(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        vec![
            ExpressionBranch::new(to_b.clone(), literal(amounts[1])),
            ExpressionBranch::new(to_a.clone(), literal(amounts[0])),
        ],
    )
    .expect("valid expression partition");
    let rule = RuleDefinition::new(
        source.clone(),
        water.clone(),
        literal(10.0),
        disposition,
        [],
    )
    .expect("valid rule");
    let bindings = ExecutionBindings::new(
        [
            TransferBranchBinding::new(source.clone(), water.clone(), to_a, sink_a),
            TransferBranchBinding::new(source, water.clone(), to_b, sink_b),
        ],
        [],
    )
    .expect("valid bindings");
    ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("valid projections"))
        .with_rules(vec![rule])
        .with_execution_bindings(bindings)
        .with_units(vec![(water, UnitId::parse("m3").expect("valid unit"))])
        .build()
        .expect("valid artifact")
}

#[test]
fn expression_partition_gives_each_named_branch_its_computed_amount() {
    let artifact = artifact([2.0, 3.0]);
    let log = execute_model(&artifact, RunId::from_bytes([0x21; 16])).expect("valid run");
    let water = SubstanceId::parse("water").expect("valid substance");

    assert_eq!(log.transfers().len(), 2);
    assert_eq!(log.transfers()[0].target().id(), &id("sink-a"));
    assert_eq!(
        log.transfers()[0].amounts().amount(&water),
        ValueState::Present(NonNegativeAmount::try_from(2.0).expect("valid amount"))
    );
    assert_eq!(log.transfers()[1].target().id(), &id("sink-b"));
    assert_eq!(
        log.transfers()[1].amounts().amount(&water),
        ValueState::Present(NonNegativeAmount::try_from(3.0).expect("valid amount"))
    );
    let replay = replay_with_artifact(&log, &artifact).expect("valid replay");
    assert_eq!(
        replay.final_state().finite_stock(&id("source"), &water),
        ValueState::Present(NonNegativeAmount::try_from(5.0).expect("valid amount"))
    );
}

#[test]
fn expression_partition_conservation_rejects_aggregate_overdraw() {
    let artifact = artifact([6.0, 5.0]);
    let error = execute_model(&artifact, RunId::from_bytes([0x22; 16]))
        .expect_err("computed partition must not fabricate stock");

    assert!(matches!(
        error,
        ExecutionError::RuleOverdraw {
            ref compartment,
            ref substance,
            timestep,
            available_bits,
            requested_bits,
        } if compartment == &id("source")
            && substance == &SubstanceId::parse("water").expect("valid substance")
            && timestep == TimestepIndex::new(0)
            && available_bits == 10.0_f64.to_bits()
            && requested_bits == 11.0_f64.to_bits()
    ));
}

#[test]
fn expression_partition_roundtrips_in_canonical_branch_order() {
    let partition = PartitionExpr::expression_partition(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        vec![
            ExpressionBranch::new(branch("to-b"), literal(3.0)),
            ExpressionBranch::new(branch("to-a"), literal(2.0)),
        ],
    )
    .expect("valid expression partition");

    let encoded = serde_json::to_value(&partition).expect("serializable partition");
    assert_eq!(encoded["partition"]["kind"], "expression_partition");
    assert_eq!(encoded["partition"]["branches"][0]["branch"], "to-a");
    assert_eq!(encoded["partition"]["branches"][1]["branch"], "to-b");
    let decoded: PartitionExpr = serde_json::from_value(encoded).expect("deserializable partition");
    assert_eq!(decoded, partition);
}
