#![allow(clippy::expect_used)]

use incidence_core::dense_projection::DenseTransferProjection;
use incidence_core::endpoints::FiniteCompartment;
use incidence_core::execution::execute_model;
use incidence_core::execution_bindings::{ExecutionBindings, TransferBranchBinding};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::ledger::{RunId, replay_with_artifact};
use incidence_core::model_artifact::{
    ModelArtifact, Quantum, RuleDefinition, SubstanceUnit, UnitId,
};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::partition_expression::{ExpressionBranch, PartitionExpr};
use incidence_core::presence::ValueState;
use incidence_core::projection::{AuthoritativeFactSelector, ProjectionSet};
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::TransferBranchId;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};
use incidence_core::versions::RuleIrVersion;

fn compartment(value: &str) -> CompartmentId {
    CompartmentId::parse(value).expect("valid compartment")
}

fn branch(value: &str) -> TransferBranchId {
    TransferBranchId::parse(value).expect("valid branch")
}

fn literal(value: f64) -> RuleExpr {
    RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value)
        .expect("valid literal")
}

fn artifact_with_last_timestep(
    quantum_value: f64,
    source_stock: f64,
    sink_a_stock: f64,
    sink_b_stock: f64,
    branches: [f64; 2],
    last_timestep: u64,
) -> ModelArtifact {
    let source = compartment("source");
    let sink_a = compartment("sink-a");
    let sink_b = compartment("sink-b");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(source.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(sink_a.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(sink_b.clone())),
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
        [
            (source.clone(), source_stock),
            (sink_a.clone(), sink_a_stock),
            (sink_b.clone(), sink_b_stock),
        ]
        .map(|(id, value)| {
            (
                id,
                SparseSubstanceVector::new(
                    &registry,
                    [(
                        water.clone(),
                        NonNegativeAmount::try_from(value).expect("valid amount"),
                    )],
                )
                .expect("valid stock vector"),
            )
        }),
    )
    .expect("valid stocks");
    let to_a = branch("to-a");
    let to_b = branch("to-b");
    let disposition = PartitionExpr::expression_partition(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        vec![
            ExpressionBranch::new(to_a.clone(), literal(branches[0])),
            ExpressionBranch::new(to_b.clone(), literal(branches[1])),
        ],
    )
    .expect("valid partition");
    let rule = RuleDefinition::new(
        source.clone(),
        water.clone(),
        literal(source_stock),
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
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("valid duration"),
    );
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(last_timestep))
        .expect("valid horizon");
    ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("valid projections"))
        .with_rules(vec![rule])
        .with_execution_bindings(bindings)
        .with_units(vec![(
            water,
            SubstanceUnit::new(
                UnitId::parse("m3").expect("valid unit"),
                Quantum::try_from(quantum_value).expect("valid quantum"),
            ),
        )])
        .build()
        .expect("valid artifact")
}

fn artifact(
    quantum_value: f64,
    source_stock: f64,
    sink_a_stock: f64,
    sink_b_stock: f64,
    branches: [f64; 2],
) -> ModelArtifact {
    artifact_with_last_timestep(
        quantum_value,
        source_stock,
        sink_a_stock,
        sink_b_stock,
        branches,
        0,
    )
}

#[test]
fn branch_amount_is_floored_to_a_whole_multiple() {
    let artifact = artifact(1.0, 10.0, 0.0, 0.0, [2.9, 3.1]);
    let log = execute_model(&artifact, RunId::from_bytes([0x41; 16])).expect("valid run");
    let water = SubstanceId::parse("water").expect("valid substance");

    assert_eq!(
        log.transfers()[0].amounts().amount(&water),
        ValueState::Present(2.0.try_into().expect("amount"))
    );
    assert_eq!(
        log.transfers()[1].amounts().amount(&water),
        ValueState::Present(3.0.try_into().expect("amount"))
    );
    let replay = replay_with_artifact(&log, &artifact).expect("valid replay");
    assert_eq!(
        replay
            .final_state()
            .finite_stock(&compartment("source"), &water),
        ValueState::Present(5.0.try_into().expect("amount"))
    );
}

#[test]
fn residual_is_exact_and_non_negative() {
    let artifact = artifact(0.125, 1.0, 0.0, 0.0, [0.375, 0.625]);
    let log = execute_model(&artifact, RunId::from_bytes([0x42; 16])).expect("valid run");
    let water = SubstanceId::parse("water").expect("valid substance");
    let replay = replay_with_artifact(&log, &artifact).expect("valid replay");

    assert_eq!(
        replay
            .final_state()
            .finite_stock(&compartment("source"), &water),
        ValueState::Present(0.0.try_into().expect("amount"))
    );
}

#[test]
fn conserved_total_is_bit_stable_across_credits() {
    let quantum = 34.452579129800505;
    let first = 499_493.0 * quantum;
    let second = 416_426.0 * quantum;
    let artifact = artifact(
        quantum,
        2.139_938_136_353_314e16,
        29.0 * quantum,
        202_059.0 * quantum,
        [first, second],
    );
    let water = SubstanceId::parse("water").expect("valid substance");
    let log = execute_model(&artifact, RunId::from_bytes([0x43; 16])).expect("valid run");
    let replay = replay_with_artifact(&log, &artifact).expect("valid replay");
    let totals = replay.conservation_totals(&water).expect("reducible total");
    let ValueState::Present(totals) = totals else {
        panic!("water must be modelled");
    };

    assert!(totals.closes_bit_identically());
}

#[test]
fn whole_count_residual_is_available_on_the_following_timestep() {
    let artifact = artifact_with_last_timestep(1.0, 10.0, 0.0, 0.0, [2.9, 3.1], 1);
    let water = SubstanceId::parse("water").expect("valid substance");
    let log = execute_model(&artifact, RunId::from_bytes([0x44; 16])).expect("valid run");
    let replay = replay_with_artifact(&log, &artifact).expect("valid replay");

    assert_eq!(log.transfers().len(), 4);
    assert_eq!(
        replay
            .final_state()
            .finite_stock(&compartment("source"), &water),
        ValueState::Present(0.0.try_into().expect("amount"))
    );
}

#[test]
fn sub_quantum_modelled_output_is_present_zero() {
    let artifact = artifact(1.0, 1.0, 0.0, 0.0, [0.9, 0.0]);
    let water = SubstanceId::parse("water").expect("valid substance");
    let log = execute_model(&artifact, RunId::from_bytes([0x45; 16])).expect("valid run");
    let projection = DenseTransferProjection::from_log_with_artifact(
        &log,
        &artifact,
        AuthoritativeFactSelector::OutgoingTransferAmount {
            compartment: compartment("source"),
            substance: water.clone(),
        },
    )
    .expect("valid projection");

    assert_eq!(
        projection.value_at(TimestepIndex::new(0)),
        ValueState::Present(0.0.try_into().expect("amount"))
    );
    assert_eq!(
        projection.value_at(TimestepIndex::new(1)),
        ValueState::Absent
    );
    let sediment = SubstanceId::parse("sediment").expect("valid substance");
    let not_modelled = DenseTransferProjection::from_log_with_artifact(
        &log,
        &artifact,
        AuthoritativeFactSelector::OutgoingTransferAmount {
            compartment: compartment("source"),
            substance: sediment,
        },
    )
    .expect("valid projection");
    assert_eq!(
        not_modelled.value_at(TimestepIndex::new(0)),
        ValueState::NotModelled
    );
    assert_eq!(
        artifact.quantum(&water).expect("declared quantum").value(),
        1.0
    );
}

#[test]
fn repeated_quantum_execution_has_identical_log_digest() {
    let artifact = artifact(0.25, 10.0, 0.0, 0.0, [2.9, 3.1]);
    let run_id = RunId::from_bytes([0x46; 16]);
    let first = execute_model(&artifact, run_id).expect("first run");
    let second = execute_model(&artifact, run_id).expect("second run");

    assert_eq!(first.digest(), second.digest());
}

#[test]
fn each_computed_branch_flooring_error_is_below_one_quantum_across_a_range() {
    let quantum = 0.125;
    for index in 1_u64..=128 {
        let target = (index as f64).mul_add(0.137, 0.019);
        let artifact = artifact(quantum, 100.0, 0.0, 0.0, [target, 0.0]);
        let water = SubstanceId::parse("water").expect("valid substance");
        let log = execute_model(
            &artifact,
            RunId::from_bytes(index.to_be_bytes().repeat(2).try_into().expect("run id")),
        )
        .expect("valid run");
        let represented = log
            .transfers()
            .iter()
            .find(|transfer| transfer.target().id() == &compartment("sink-a"))
            .map(|transfer| match transfer.amounts().amount(&water) {
                ValueState::Present(amount) => amount.value(),
                state => panic!("modelled branch amount must be present: {state:?}"),
            })
            .unwrap_or(0.0);

        assert!(represented <= target, "{represented} > {target}");
        assert!(
            target - represented < quantum,
            "target={target}, represented={represented}"
        );
    }
}
