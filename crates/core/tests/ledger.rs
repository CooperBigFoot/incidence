#![allow(clippy::expect_used)]

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::ledger::{
    AuthoritativeLog, CompletenessReader, Genesis, QuantumCount, Record, ReplayError, RunId,
    RunStatus, Transfer,
};
use incidence_core::model_artifact::{
    ModelArtifact, ModelArtifactArchive, Quantum, SubstanceUnit, UnitId,
};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::presence::ValueState;
use incidence_core::projection::ProjectionSet;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};

pub(crate) fn id(value: &str) -> CompartmentId {
    CompartmentId::parse(value).expect("valid id")
}
pub(crate) fn fixture(salt: bool, forcing_stock: f64) -> ModelArtifact {
    let store = id("store");
    let route = id("route");
    let outside_in = id("outside_in");
    let outside_out = id("outside_out");
    let topology = Topology::new(
        [
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside_in.clone())),
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside_out.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(store.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(route.clone())),
        ],
        [
            DirectedConnection::new(outside_in, store.clone()),
            DirectedConnection::new(store.clone(), route.clone()),
            DirectedConnection::new(route, outside_out),
        ],
    )
    .expect("topology");
    let water = SubstanceId::parse("water").expect("water");
    let salt_id = SubstanceId::parse("salt").expect("salt");
    let registry = SubstanceRegistry::new(if salt {
        vec![water.clone(), salt_id.clone()]
    } else {
        vec![water.clone()]
    })
    .expect("registry");
    let vector = SparseSubstanceVector::new(
        &registry,
        [(
            water.clone(),
            NonNegativeAmount::try_from(forcing_stock).expect("amount"),
        )],
    )
    .expect("vector");
    let stocks = InitialStocks::new(&topology, &registry, [(store, vector)]).expect("stocks");
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(3)).expect("horizon");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("duration"),
    );
    let mut units = vec![(
        water,
        SubstanceUnit::new(
            UnitId::parse("kg").expect("unit"),
            Quantum::try_from(1.0e-6).expect("valid quantum"),
        ),
    )];
    if salt {
        units.push((
            salt_id,
            SubstanceUnit::new(
                UnitId::parse("kg").expect("unit"),
                Quantum::try_from(1.0e-6).expect("valid quantum"),
            ),
        ));
    }
    ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("projections"))
        .with_units(units)
        .build()
        .expect("artifact")
}
pub(crate) fn endpoint(artifact: &ModelArtifact, name: &str) -> TopologyEndpoint {
    artifact
        .topology()
        .endpoint(&id(name))
        .expect("endpoint")
        .clone()
}

#[test]
fn replay_is_dense_exact_and_seal_distinguishes_prefix() {
    let artifact = fixture(false, 10.0);
    let registry = artifact.registry();
    let water = SubstanceId::parse("water").expect("water");
    let amount = SparseSubstanceVector::new(
        registry,
        [(
            water.clone(),
            NonNegativeAmount::try_from(3.0).expect("amount"),
        )],
    )
    .expect("vector");
    let mut log = AuthoritativeLog::for_run(RunId::from_bytes([7; 16]), &artifact);
    log.append(
        Transfer::new(
            TimestepIndex::new(1),
            endpoint(&artifact, "store"),
            endpoint(&artifact, "route"),
            amount,
            [(
                water.clone(),
                QuantumCount::try_from(3000000).expect("count"),
            )],
        )
        .expect("count-bound transfer"),
    )
    .expect("append");
    let mut archive = ModelArtifactArchive::new();
    archive.insert(artifact).expect("archive");
    let prefix = CompletenessReader::read(&log, &archive).expect("replay");
    assert!(matches!(prefix.status(), RunStatus::ResumablePrefix { .. }));
    assert_eq!(prefix.state_at(TimestepIndex::new(2)), ValueState::Absent);
    assert_eq!(
        prefix.final_state().finite_stock(&id("store"), &water),
        ValueState::Present(NonNegativeAmount::try_from(7.0).expect("amount"))
    );
    log.seal(TimestepIndex::new(3)).expect("seal");
    let completed = CompletenessReader::read_completed(&log, &archive).expect("completed");
    assert_eq!(
        completed.status(),
        RunStatus::Complete {
            final_timestep: TimestepIndex::new(3)
        }
    );
    assert_eq!(
        completed.state_at(TimestepIndex::new(2)),
        ValueState::Present(completed.final_state())
    );
    let records = log.records().collect::<Vec<_>>();
    let truncated = AuthoritativeLog::from_records(records[..records.len() - 1].iter().cloned())
        .expect("prefix records");
    assert!(matches!(
        CompletenessReader::read(&truncated, &archive)
            .expect("prefix")
            .status(),
        RunStatus::ResumablePrefix { .. }
    ));
}

#[test]
fn unmodelled_is_not_zero_and_digest_mismatch_is_rejected() {
    let artifact = fixture(false, 10.0);
    let changed = fixture(true, 10.0);
    let log = AuthoritativeLog::new(Genesis::for_run(RunId::from_bytes([7; 16]), &artifact));
    let replay = incidence_core::ledger::replay_with_artifact(&log, &artifact).expect("replay");
    let salt = SubstanceId::parse("salt").expect("salt");
    assert_eq!(
        replay.final_state().finite_stock(&id("store"), &salt),
        ValueState::NotModelled
    );
    assert!(matches!(
        incidence_core::ledger::replay_with_artifact(&log, &changed),
        Err(ReplayError::ModelDigestMismatch { .. })
    ));
}

#[test]
fn zero_transfer_is_authoritative_and_seal_authenticates_it() {
    let artifact = fixture(false, 10.0);
    let empty = SparseSubstanceVector::new(artifact.registry(), []).expect("empty");
    let mut without = AuthoritativeLog::for_run(RunId::from_bytes([7; 16]), &artifact);
    let first = without.digest();
    without
        .append(
            Transfer::new(
                TimestepIndex::new(0),
                endpoint(&artifact, "store"),
                endpoint(&artifact, "route"),
                empty,
                [],
            )
            .expect("count-bound transfer"),
        )
        .expect("append");
    assert_ne!(first, without.digest());
    assert_eq!(without.transfer_count(), 1);
    without.seal(TimestepIndex::new(3)).expect("seal");
    let mut records = without.records().collect::<Vec<_>>();
    records.insert(
        1,
        Record::Transfer(match &records[1] {
            Record::Transfer(value) => value.clone(),
            _ => panic!("transfer"),
        }),
    );
    assert!(AuthoritativeLog::from_records(records).is_err());
}
