#![allow(clippy::expect_used)]

use incidence_core::endpoints::FiniteCompartment;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::ledger::{AuthoritativeLog, RunId, Transfer, replay_with_artifact};
use incidence_core::model_artifact::{ModelArtifact, UnitId};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::projection::ProjectionSet;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};

fn id(value: &str) -> CompartmentId {
    CompartmentId::parse(value).expect("valid compartment id")
}

#[test]
fn replay_rejects_a_transfer_whose_binary64_updates_do_not_close() {
    let source = id("a_source");
    let target = id("b_target");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(source.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(target.clone())),
        ],
        [DirectedConnection::new(source.clone(), target.clone())],
    )
    .expect("valid topology");
    let water = SubstanceId::parse("water").expect("valid substance id");
    let registry = SubstanceRegistry::new([water.clone()]).expect("valid registry");
    let source_stock = SparseSubstanceVector::new(
        &registry,
        [(
            water.clone(),
            NonNegativeAmount::try_from(1.0e16).expect("valid amount"),
        )],
    )
    .expect("valid source stock");
    let target_stock = SparseSubstanceVector::new(
        &registry,
        [(
            water.clone(),
            NonNegativeAmount::try_from(1.0).expect("valid amount"),
        )],
    )
    .expect("valid target stock");
    let stocks = InitialStocks::new(
        &topology,
        &registry,
        [
            (source.clone(), source_stock),
            (target.clone(), target_stock),
        ],
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
            UnitId::parse("kg").expect("valid unit"),
        )])
        .build()
        .expect("valid artifact");
    let amount = SparseSubstanceVector::new(
        artifact.registry(),
        [(
            water,
            NonNegativeAmount::try_from(1.0).expect("valid amount"),
        )],
    )
    .expect("valid transfer amount");
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([31; 16]), &artifact);
    log.append(Transfer::new(
        TimestepIndex::new(0),
        artifact
            .topology()
            .endpoint(&source)
            .expect("source endpoint")
            .clone(),
        artifact
            .topology()
            .endpoint(&target)
            .expect("target endpoint")
            .clone(),
        amount,
    ))
    .expect("structurally valid transfer");

    assert!(replay_with_artifact(&log, &artifact).is_err());
}
