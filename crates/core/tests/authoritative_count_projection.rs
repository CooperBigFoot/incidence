#![allow(clippy::expect_used)]

use incidence_core::dense_projection::{DenseTransferCountProjection, DenseTransferProjection};
use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::ledger::{
    AuthoritativeLog, QuantumAmount, QuantumCount, RunId, Transfer, replay_with_artifact,
};
use incidence_core::model_artifact::{ModelArtifact, Quantum, SubstanceUnit, UnitId};
use incidence_core::presence::ValueState;
use incidence_core::projection::{AuthoritativeFactSelector, ProjectionSet};
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};

const INDIVIDUAL_COUNT: u64 = 4_394_222_044_288_838;
const MERGED_COUNT: u64 = 8_788_444_088_577_676;

#[test]
fn authoritative_count_projection_does_not_redecode_an_ambiguous_f64_sum() {
    let outside = CompartmentId::parse("outside").expect("outside");
    let store = CompartmentId::parse("store").expect("store");
    let topology = Topology::new(
        [
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(store.clone())),
        ],
        [DirectedConnection::new(outside.clone(), store.clone())],
    )
    .expect("topology");
    let water = SubstanceId::parse("water").expect("water");
    let registry = SubstanceRegistry::new([water.clone()]).expect("registry");
    let stocks = InitialStocks::new(&topology, &registry, []).expect("stocks");
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
                Quantum::try_from(1.0e-6).expect("quantum"),
            ),
        )])
        .build()
        .expect("artifact");
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([0x61; 16]), &artifact);
    for _ in 0..2 {
        let transfer = Transfer::new(
            TimestepIndex::new(0),
            artifact
                .topology()
                .endpoint(&outside)
                .expect("outside")
                .clone(),
            artifact.topology().endpoint(&store).expect("store").clone(),
            artifact.registry(),
            [(
                water.clone(),
                QuantumAmount::new(
                    artifact.quantum(&water).expect("quantum"),
                    QuantumCount::try_from(INDIVIDUAL_COUNT).expect("count"),
                )
                .expect("amount"),
            )],
        )
        .expect("transfer");
        log.append(transfer).expect("append");
    }
    log.seal(TimestepIndex::new(0)).expect("seal");

    let selector = AuthoritativeFactSelector::IncomingTransferAmount {
        compartment: store.clone(),
        substance: water.clone(),
    };
    let public = DenseTransferProjection::from_log_with_artifact(&log, &artifact, selector.clone())
        .expect("public projection");
    let ValueState::Present(projected) = public.value_at(TimestepIndex::new(0)) else {
        panic!("expected present public value");
    };
    assert_eq!(projected.value(), 8_788_444_088.577_675);
    assert_ne!((projected.value() / 1.0e-6) as u64, MERGED_COUNT);

    let counts = DenseTransferCountProjection::from_log_with_artifact(&log, &artifact, selector)
        .expect("authoritative count projection");
    let ValueState::Present(merged) = counts.value_at(TimestepIndex::new(0)) else {
        panic!("expected present authoritative count");
    };
    assert_eq!(merged.value(), MERGED_COUNT);
    let replay = replay_with_artifact(&log, &artifact).expect("replay");
    assert_eq!(
        replay.final_state().finite_quantum_count(&store, &water),
        Some(MERGED_COUNT)
    );
}
