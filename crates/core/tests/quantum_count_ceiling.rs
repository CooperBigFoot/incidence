#![allow(clippy::expect_used)]

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::ledger::{
    AuthoritativeLog, QuantumCount, ReplayError, RunId, Transfer, replay_with_artifact,
};
use incidence_core::model_artifact::{ModelArtifact, Quantum, SubstanceUnit, UnitId};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::projection::ProjectionSet;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};

#[test]
fn replay_refuses_an_endpoint_count_above_the_exact_ceiling() {
    let outside = CompartmentId::parse("outside").expect("valid boundary id");
    let store = CompartmentId::parse("store").expect("valid finite id");
    let topology = Topology::new(
        [
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(store.clone())),
        ],
        [DirectedConnection::new(outside.clone(), store.clone())],
    )
    .expect("valid topology");
    let water = SubstanceId::parse("water").expect("valid substance");
    let registry = SubstanceRegistry::new([water.clone()]).expect("valid registry");
    let ceiling = 9_007_199_254_740_992.0;
    let stocks = InitialStocks::new(
        &topology,
        &registry,
        [(
            store.clone(),
            SparseSubstanceVector::new(
                &registry,
                [(
                    water.clone(),
                    NonNegativeAmount::try_from(ceiling).expect("valid ceiling stock"),
                )],
            )
            .expect("valid stock vector"),
        )],
    )
    .expect("valid stocks");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("valid duration"),
    );
    let horizon =
        RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(0)).expect("valid horizon");
    let artifact = ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("valid projections"))
        .with_units(vec![(
            water.clone(),
            SubstanceUnit::new(
                UnitId::parse("m3").expect("valid unit"),
                Quantum::try_from(1.0).expect("valid quantum"),
            ),
        )])
        .build()
        .expect("valid artifact at the exact ceiling");
    let amounts = SparseSubstanceVector::new(
        artifact.registry(),
        [(
            water,
            NonNegativeAmount::try_from(1.0).expect("valid amount"),
        )],
    )
    .expect("valid transfer amounts");
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([0x51; 16]), &artifact);
    log.append(
        Transfer::new(
            TimestepIndex::new(0),
            artifact
                .topology()
                .endpoint(&outside)
                .expect("boundary endpoint")
                .clone(),
            artifact
                .topology()
                .endpoint(&store)
                .expect("finite endpoint")
                .clone(),
            amounts,
            [(
                SubstanceId::parse("water").expect("water"),
                QuantumCount::try_from(1).expect("count"),
            )],
        )
        .expect("count-bound transfer"),
    )
    .expect("valid authoritative transfer");

    assert!(matches!(
        replay_with_artifact(&log, &artifact),
        Err(ReplayError::NonFiniteFold { compartment, .. }) if compartment == store
    ));
}
