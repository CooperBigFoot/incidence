use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::CompartmentId;
use incidence_core::topology::{
    ConnectionEndpointRole, DirectedConnection, Topology, TopologyEndpoint, TopologyError,
};

fn id(value: &str) -> CompartmentId {
    match CompartmentId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid fixture identity `{value}`: {error}"),
    }
}

fn finite(value: &str) -> TopologyEndpoint {
    TopologyEndpoint::Finite(FiniteCompartment::new(id(value)))
}

fn boundary(value: &str) -> TopologyEndpoint {
    TopologyEndpoint::Boundary(BoundaryAccount::new(id(value)))
}

fn connection(source: &str, target: &str) -> DirectedConnection {
    DirectedConnection::new(id(source), id(target))
}

fn texts(ids: &[CompartmentId]) -> Vec<&str> {
    ids.iter().map(CompartmentId::as_str).collect()
}

#[test]
fn accepts_boundary_connections_and_preserves_structural_classification() {
    let topology = Topology::new(
        [boundary("catchment"), finite("reservoir"), boundary("sea")],
        [
            connection("catchment", "reservoir"),
            connection("reservoir", "sea"),
        ],
    )
    .expect("fixture topology must be valid");

    assert_eq!(topology.len(), 3);
    assert!(!topology.is_empty());
    assert_eq!(
        texts(topology.topological_order()),
        ["catchment", "reservoir", "sea"]
    );
    assert_eq!(
        topology
            .connections()
            .iter()
            .map(|connection| (connection.source().as_str(), connection.target().as_str()))
            .collect::<Vec<_>>(),
        [("catchment", "reservoir"), ("reservoir", "sea")]
    );
    assert!(matches!(
        topology.endpoint(&id("catchment")),
        Some(TopologyEndpoint::Boundary(account))
            if account.id().as_str() == "catchment"
    ));
    assert!(matches!(
        topology.endpoint(&id("reservoir")),
        Some(TopologyEndpoint::Finite(compartment))
            if compartment.id().as_str() == "reservoir"
    ));
    assert!(matches!(
        topology.endpoint(&id("sea")),
        Some(TopologyEndpoint::Boundary(account)) if account.id().as_str() == "sea"
    ));
    assert_eq!(topology.endpoint(&id("unknown")), None);
    assert_eq!(
        topology
            .endpoints()
            .map(|endpoint| endpoint.id().as_str())
            .collect::<Vec<_>>(),
        ["catchment", "reservoir", "sea"]
    );
}

#[test]
fn permits_an_empty_topology() {
    let topology = Topology::new(
        Vec::<TopologyEndpoint>::new(),
        Vec::<DirectedConnection>::new(),
    )
    .expect("fixture topology must be valid");

    assert_eq!(topology.len(), 0);
    assert!(topology.is_empty());
    assert_eq!(topology.endpoints().next(), None);
    assert_eq!(topology.connections(), []);
    assert_eq!(topology.topological_order(), []);
}

#[test]
fn rejects_duplicate_identity_across_endpoint_classifications() {
    let result = Topology::new(
        [finite("reservoir"), boundary("reservoir")],
        Vec::<DirectedConnection>::new(),
    );

    assert_eq!(
        result,
        Err(TopologyError::DuplicateEndpoint {
            endpoint: id("reservoir"),
        })
    );
    assert_eq!(
        result
            .expect_err("fixture topology must be rejected")
            .to_string(),
        "duplicate topology endpoint identity `reservoir`"
    );
}

#[test]
fn rejects_duplicate_directed_connection() {
    let result = Topology::new(
        [finite("a"), finite("b")],
        [connection("a", "b"), connection("a", "b")],
    );

    assert_eq!(
        result,
        Err(TopologyError::DuplicateConnection {
            connection_source: id("a"),
            connection_target: id("b"),
        })
    );
    assert_eq!(
        result
            .expect_err("fixture topology must be rejected")
            .to_string(),
        "duplicate directed connection `a` -> `b`"
    );
}

#[test]
fn rejects_unknown_source_endpoint() {
    let result = Topology::new([finite("b")], [connection("undeclared", "b")]);

    assert_eq!(
        result,
        Err(TopologyError::UnknownConnectionEndpoint {
            role: ConnectionEndpointRole::Source,
            endpoint: id("undeclared"),
        })
    );
    assert_eq!(
        result
            .expect_err("fixture topology must be rejected")
            .to_string(),
        "directed connection source endpoint `undeclared` is not declared in the topology"
    );
}

#[test]
fn rejects_unknown_target_endpoint() {
    let result = Topology::new([finite("a")], [connection("a", "undeclared")]);

    assert_eq!(
        result,
        Err(TopologyError::UnknownConnectionEndpoint {
            role: ConnectionEndpointRole::Target,
            endpoint: id("undeclared"),
        })
    );
    assert_eq!(
        result
            .expect_err("fixture topology must be rejected")
            .to_string(),
        "directed connection target endpoint `undeclared` is not declared in the topology"
    );
}

#[test]
fn rejects_cycle_with_deterministic_diagnostics() {
    let result = Topology::new(
        [finite("c"), finite("a"), finite("b")],
        [
            connection("c", "a"),
            connection("a", "b"),
            connection("b", "c"),
        ],
    );

    assert_eq!(
        result,
        Err(TopologyError::Cycle {
            blocked_endpoint_ids: vec![id("a"), id("b"), id("c")],
        })
    );
    assert_eq!(
        result
            .expect_err("fixture topology must be rejected")
            .to_string(),
        "topology contains a directed cycle; blocked endpoint identities: [CompartmentId(\"a\"), CompartmentId(\"b\"), CompartmentId(\"c\")]"
    );
}

#[test]
fn topological_order_and_storage_are_independent_of_insertion_order() {
    let first = Topology::new(
        [finite("d"), finite("c"), finite("b"), finite("a")],
        [
            connection("c", "d"),
            connection("b", "c"),
            connection("a", "c"),
        ],
    )
    .expect("fixture topology must be valid");
    let second = Topology::new(
        [finite("a"), finite("b"), finite("c"), finite("d")],
        [
            connection("a", "c"),
            connection("b", "c"),
            connection("c", "d"),
        ],
    )
    .expect("fixture topology must be valid");

    assert_eq!(first, second);
    assert_eq!(texts(first.topological_order()), ["a", "b", "c", "d"]);
    assert_eq!(texts(second.topological_order()), ["a", "b", "c", "d"]);
    assert_eq!(
        first
            .endpoints()
            .map(|endpoint| endpoint.id().as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c", "d"]
    );
    assert_eq!(
        second
            .endpoints()
            .map(|endpoint| endpoint.id().as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c", "d"]
    );
    assert_eq!(
        first
            .connections()
            .iter()
            .map(|connection| (connection.source().as_str(), connection.target().as_str()))
            .collect::<Vec<_>>(),
        [("a", "c"), ("b", "c"), ("c", "d")]
    );
    assert_eq!(
        second
            .connections()
            .iter()
            .map(|connection| (connection.source().as_str(), connection.target().as_str()))
            .collect::<Vec<_>>(),
        [("a", "c"), ("b", "c"), ("c", "d")]
    );
}
