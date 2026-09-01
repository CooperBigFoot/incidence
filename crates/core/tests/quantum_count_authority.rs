#![allow(clippy::expect_used)]

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::execution::execute_model;
use incidence_core::execution_bindings::{ExecutionBindings, TransferBranchBinding};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::ledger::{
    AuthoritativeLog, QuantumCount, RunId, Transfer, replay_with_artifact,
};
use incidence_core::model_artifact::{
    ModelArtifact, Quantum, RuleDefinition, SubstanceUnit, UnitId,
};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::partition_expression::PartitionExpr;
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

const COLLIDING_COUNT: u64 = 8_925_151_700_786_600;

fn fixture() -> (ModelArtifact, CompartmentId, CompartmentId, SubstanceId) {
    let outside = CompartmentId::parse("outside").expect("boundary id");
    let store = CompartmentId::parse("store").expect("finite id");
    let topology = Topology::new(
        [
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(store.clone())),
        ],
        [DirectedConnection::new(outside.clone(), store.clone())],
    )
    .expect("valid topology");
    let water = SubstanceId::parse("water").expect("substance");
    let registry = SubstanceRegistry::new([water.clone()]).expect("registry");
    let stocks = InitialStocks::new(&topology, &registry, []).expect("zero stocks");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("duration"),
    );
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(0)).expect("horizon");
    let artifact = ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("projections"))
        .with_units(vec![(
            water.clone(),
            SubstanceUnit::new(
                UnitId::parse("m3").expect("unit"),
                Quantum::try_from(0.001).expect("quantum"),
            ),
        )])
        .build()
        .expect("artifact");
    (artifact, outside, store, water)
}

fn log_for_count(
    artifact: &ModelArtifact,
    outside: &CompartmentId,
    store: &CompartmentId,
    water: &SubstanceId,
    count: u64,
) -> AuthoritativeLog {
    let projected = (count as f64) * 0.001;
    let amounts = SparseSubstanceVector::new(
        artifact.registry(),
        [(
            water.clone(),
            NonNegativeAmount::try_from(projected).expect("amount"),
        )],
    )
    .expect("amounts");
    let transfer = Transfer::new(
        TimestepIndex::new(0),
        artifact
            .topology()
            .endpoint(outside)
            .expect("outside")
            .clone(),
        artifact.topology().endpoint(store).expect("store").clone(),
        amounts,
        [(
            water.clone(),
            QuantumCount::try_from(count).expect("count below ceiling"),
        )],
    )
    .expect("count-bound transfer");
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([0x52; 16]), artifact);
    log.append(transfer).expect("append");
    log
}

#[test]
fn colliding_public_values_keep_distinct_authoritative_transfer_counts() {
    let (artifact, outside, store, water) = fixture();
    let higher = log_for_count(&artifact, &outside, &store, &water, COLLIDING_COUNT);
    let lower = log_for_count(&artifact, &outside, &store, &water, COLLIDING_COUNT - 1);

    assert_eq!(
        higher.transfers()[0].amounts().amount(&water),
        lower.transfers()[0].amounts().amount(&water)
    );
    assert_ne!(higher.canonical_bytes(), lower.canonical_bytes());
    assert_ne!(higher.digest(), lower.digest());

    let replay = replay_with_artifact(&higher, &artifact).expect("count-authoritative replay");
    assert_eq!(
        replay.final_state().finite_quantum_count(&store, &water),
        Some(COLLIDING_COUNT)
    );
}

#[test]
fn release_all_carries_a_colliding_count_through_execution_log_and_replay() {
    let source_a = CompartmentId::parse("source-a").expect("source a");
    let source_b = CompartmentId::parse("source-b").expect("source b");
    let aggregator = CompartmentId::parse("aggregator").expect("aggregator");
    let outside = CompartmentId::parse("outside").expect("outside");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(source_a.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(source_b.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(aggregator.clone())),
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside.clone())),
        ],
        [
            DirectedConnection::new(source_a.clone(), aggregator.clone()),
            DirectedConnection::new(source_b.clone(), aggregator.clone()),
            DirectedConnection::new(aggregator.clone(), outside.clone()),
        ],
    )
    .expect("topology");
    let water = SubstanceId::parse("water").expect("water");
    let registry = SubstanceRegistry::new([water.clone()]).expect("registry");
    let half_count = COLLIDING_COUNT / 2;
    let half_value = (half_count as f64) * 0.001;
    let stocks = InitialStocks::new(
        &topology,
        &registry,
        [source_a.clone(), source_b.clone()].map(|source| {
            (
                source,
                SparseSubstanceVector::new(
                    &registry,
                    [(
                        water.clone(),
                        NonNegativeAmount::try_from(half_value).expect("half stock"),
                    )],
                )
                .expect("stock vector"),
            )
        }),
    )
    .expect("stocks");
    let literal = |value| {
        RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value).expect("literal")
    };
    let branch_a = TransferBranchId::parse("from-a").expect("branch a");
    let branch_b = TransferBranchId::parse("from-b").expect("branch b");
    let branch_out = TransferBranchId::parse("to-outside").expect("branch outside");
    let release = |source: CompartmentId, branch: TransferBranchId, expression| {
        RuleDefinition::new(
            source,
            water.clone(),
            literal(expression),
            PartitionExpr::release_all(RuleIrVersion::V1, NumericalSemanticsVersion::V1, branch),
            [],
        )
        .expect("release-all rule")
    };
    let rules = vec![
        release(source_a.clone(), branch_a.clone(), half_value),
        release(source_b.clone(), branch_b.clone(), half_value),
        release(
            aggregator.clone(),
            branch_out.clone(),
            (COLLIDING_COUNT as f64) * 0.001,
        ),
    ];
    let bindings = ExecutionBindings::new(
        [
            TransferBranchBinding::new(
                source_a.clone(),
                water.clone(),
                branch_a,
                aggregator.clone(),
            ),
            TransferBranchBinding::new(
                source_b.clone(),
                water.clone(),
                branch_b,
                aggregator.clone(),
            ),
            TransferBranchBinding::new(aggregator.clone(), water.clone(), branch_out, outside),
        ],
        [],
    )
    .expect("bindings");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("duration"),
    );
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(0)).expect("horizon");
    let artifact = ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("projections"))
        .with_rules(rules)
        .with_execution_bindings(bindings)
        .with_units(vec![(
            water.clone(),
            SubstanceUnit::new(
                UnitId::parse("m3").expect("unit"),
                Quantum::try_from(0.001).expect("quantum"),
            ),
        )])
        .build()
        .expect("artifact with unambiguous split initial stocks");

    let log = execute_model(&artifact, RunId::from_bytes([0x53; 16])).expect("execution");
    let outgoing = log
        .transfers()
        .iter()
        .find(|transfer| transfer.source().id() == &aggregator)
        .expect("aggregator ReleaseAll transfer");
    assert_eq!(
        outgoing.quantum_count(&water).map(QuantumCount::value),
        Some(COLLIDING_COUNT)
    );
    let replay = replay_with_artifact(&log, &artifact).expect("count-authoritative replay");
    assert_eq!(
        replay
            .final_state()
            .finite_quantum_count(&aggregator, &water),
        Some(0)
    );
}
