#![allow(clippy::expect_used)]

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::model_artifact::{ModelArtifact, Quantum, SubstanceUnit, UnitId};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::projection::ProjectionSet;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};

pub(crate) fn compartment(value: &str) -> CompartmentId {
    CompartmentId::parse(value).expect("valid compartment")
}

pub(crate) fn substance(value: &str) -> SubstanceId {
    SubstanceId::parse(value).expect("valid substance")
}

pub(crate) fn amount(value: f64) -> NonNegativeAmount {
    NonNegativeAmount::try_from(value).expect("valid amount")
}

pub(crate) fn fixture() -> ModelArtifact {
    let store = compartment("store");
    let route = compartment("route");
    let outside = compartment("outside");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(store.clone())),
            TopologyEndpoint::Finite(FiniteCompartment::new(route.clone())),
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside.clone())),
        ],
        [
            DirectedConnection::new(store.clone(), route),
            DirectedConnection::new(store.clone(), outside),
        ],
    )
    .expect("valid topology");
    let water = substance("water");
    let salt = substance("salt");
    let registry = SubstanceRegistry::new([water.clone(), salt.clone()]).expect("valid registry");
    let stock = SparseSubstanceVector::new(
        &registry,
        [(water.clone(), amount(10.0)), (salt.clone(), amount(4.0))],
    )
    .expect("valid stock");
    let stocks = InitialStocks::new(&topology, &registry, [(store, stock)]).expect("valid stocks");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(86_400).expect("valid duration"),
    );
    let horizon =
        RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(2)).expect("valid horizon");
    ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("valid projections"))
        .with_units(vec![
            (
                water,
                SubstanceUnit::new(
                    UnitId::parse("kg").expect("valid unit"),
                    Quantum::try_from(1.0e-6).expect("valid quantum"),
                ),
            ),
            (
                salt,
                SubstanceUnit::new(
                    UnitId::parse("kg").expect("valid unit"),
                    Quantum::try_from(1.0e-6).expect("valid quantum"),
                ),
            ),
        ])
        .build()
        .expect("valid artifact")
}

pub(crate) fn finite(artifact: &ModelArtifact, value: &str) -> FiniteCompartment {
    match artifact
        .topology()
        .endpoint(&compartment(value))
        .expect("known endpoint")
    {
        TopologyEndpoint::Finite(endpoint) => endpoint.clone(),
        TopologyEndpoint::Boundary(_) => panic!("fixture endpoint must be finite"),
    }
}

pub(crate) fn endpoint(artifact: &ModelArtifact, value: &str) -> TopologyEndpoint {
    artifact
        .topology()
        .endpoint(&compartment(value))
        .expect("known endpoint")
        .clone()
}
