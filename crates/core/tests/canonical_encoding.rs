//! Black-box golden tests for canonical V1 bytes.

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};
use incidence_core::versions::{CanonicalEncodingVersion, InterpreterVersion, RuleIrVersion};

#[derive(Clone, Copy)]
enum FixtureEndpointKind {
    Finite,
    Boundary,
}

fn compartment(value: &str) -> CompartmentId {
    match CompartmentId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid authored compartment fixture: {error}"),
    }
}

fn substance(value: &str) -> SubstanceId {
    match SubstanceId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid authored substance fixture: {error}"),
    }
}

fn amount(value: f64) -> NonNegativeAmount {
    match NonNegativeAmount::try_from(value) {
        Ok(amount) => amount,
        Err(error) => panic!("invalid authored amount fixture: {error}"),
    }
}

fn registry(values: &[&str]) -> SubstanceRegistry {
    match SubstanceRegistry::new(values.iter().map(|value| substance(value))) {
        Ok(registry) => registry,
        Err(error) => panic!("invalid authored registry fixture: {error}"),
    }
}

fn sparse(registry: &SubstanceRegistry, entries: &[(&str, f64)]) -> SparseSubstanceVector {
    match SparseSubstanceVector::new(
        registry,
        entries
            .iter()
            .map(|(id, value)| (substance(id), amount(*value))),
    ) {
        Ok(vector) => vector,
        Err(error) => panic!("invalid authored sparse fixture: {error}"),
    }
}

fn topology(endpoints: &[(&str, FixtureEndpointKind)], connections: &[(&str, &str)]) -> Topology {
    let endpoints = endpoints.iter().map(|(id, kind)| {
        let id = compartment(id);
        match kind {
            FixtureEndpointKind::Finite => TopologyEndpoint::Finite(FiniteCompartment::new(id)),
            FixtureEndpointKind::Boundary => TopologyEndpoint::Boundary(BoundaryAccount::new(id)),
        }
    });
    let connections = connections
        .iter()
        .map(|(source, target)| DirectedConnection::new(compartment(source), compartment(target)));
    match Topology::new(endpoints, connections) {
        Ok(topology) => topology,
        Err(error) => panic!("invalid authored topology fixture: {error}"),
    }
}

fn stocks(
    topology: &Topology,
    registry: &SubstanceRegistry,
    entries: Vec<(CompartmentId, SparseSubstanceVector)>,
) -> InitialStocks {
    match InitialStocks::new(topology, registry, entries) {
        Ok(stocks) => stocks,
        Err(error) => panic!("invalid authored initial-stocks fixture: {error}"),
    }
}

fn calendar(origin: i64, duration: u64) -> FixedStepCalendar {
    let duration = match TimestepDuration::from_seconds(duration) {
        Ok(duration) => duration,
        Err(error) => panic!("invalid authored duration fixture: {error}"),
    };
    FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(origin)),
        duration,
    )
}

fn horizon(first: u64, last: u64) -> RunHorizon {
    match RunHorizon::new(TimestepIndex::new(first), TimestepIndex::new(last)) {
        Ok(horizon) => horizon,
        Err(error) => panic!("invalid authored horizon fixture: {error}"),
    }
}

fn encoded<T: incidence_core::canonical_encoding::CanonicalEncode>(value: &T) -> Vec<u8> {
    match CanonicalEncodingVersion::V1.encode(value) {
        Ok(bytes) => bytes,
        Err(error) => panic!("authored fixture must encode: {error}"),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn version_identities_have_distinct_exact_v1_encodings() {
    let fixtures = [
        (hex(&encoded(&RuleIrVersion::V1)), "494e4344000100010001"),
        (
            hex(&encoded(&InterpreterVersion::V1)),
            "494e4344000100020001",
        ),
        (
            hex(&encoded(&CanonicalEncodingVersion::V1)),
            "494e4344000100030001",
        ),
        (
            hex(&encoded(&NumericalSemanticsVersion::V1)),
            "494e4344000100040001",
        ),
    ];
    for (actual, expected) in &fixtures {
        assert_eq!(actual, expected);
    }
    for left in 0..fixtures.len() {
        for right in (left + 1)..fixtures.len() {
            assert_ne!(fixtures[left].0, fixtures[right].0);
        }
    }
}

#[test]
fn registry_and_sparse_views_define_canonical_constructor_order_and_zero() {
    let registry_ba = registry(&["b", "a"]);
    let registry_ab = registry(&["a", "b"]);
    let registry_golden = "494e4344000100100000000000000002000000000000000161000000000000000162";
    assert_eq!(hex(&encoded(&registry_ba)), registry_golden);
    assert_eq!(encoded(&registry_ba), encoded(&registry_ab));

    let sparse_golden = "494e434400010011000000000000000200000000000000016100000000000000016200000000000000010000000000000001623ff8000000000000";
    let negative_zero = sparse(&registry_ab, &[("a", -0.0), ("b", 1.5)]);
    let positive_zero_reversed = sparse(&registry_ba, &[("b", 1.5), ("a", 0.0)]);
    let omitted = sparse(&registry_ab, &[("b", 1.5)]);
    assert_eq!(hex(&encoded(&negative_zero)), sparse_golden);
    assert_eq!(encoded(&negative_zero), encoded(&positive_zero_reversed));
    assert_eq!(encoded(&negative_zero), encoded(&omitted));
}

#[test]
fn registry_membership_is_never_conflated_for_zero_sparse_vectors() {
    let a = encoded(&sparse(&registry(&["a"]), &[]));
    let b = encoded(&sparse(&registry(&["b"]), &[]));
    let empty = encoded(&sparse(&registry(&[]), &[]));
    assert_eq!(
        hex(&a),
        "494e43440001001100000000000000010000000000000001610000000000000000"
    );
    assert_eq!(
        hex(&b),
        "494e43440001001100000000000000010000000000000001620000000000000000"
    );
    assert_eq!(
        hex(&empty),
        "494e43440001001100000000000000000000000000000000"
    );
    assert_ne!(a, b);
    assert_ne!(a, empty);
    assert_ne!(b, empty);
}

#[test]
fn topology_golden_is_canonical_across_constructor_permutations() {
    let first = topology(
        &[
            ("a", FixtureEndpointKind::Finite),
            ("b", FixtureEndpointKind::Boundary),
        ],
        &[("a", "b")],
    );
    let second = topology(
        &[
            ("b", FixtureEndpointKind::Boundary),
            ("a", FixtureEndpointKind::Finite),
        ],
        &[("a", "b")],
    );
    let golden = "494e434400010012000000000000000200000000000000000161010000000000000001620000000000000001000000000000000161000000000000000162";
    assert_eq!(hex(&encoded(&first)), golden);
    assert_eq!(encoded(&first), encoded(&second));
}

#[test]
fn topology_endpoint_kind_and_connection_direction_are_explicit() {
    let finite_a = encoded(&topology(
        &[
            ("a", FixtureEndpointKind::Finite),
            ("b", FixtureEndpointKind::Boundary),
        ],
        &[],
    ));
    let boundary_a = encoded(&topology(
        &[
            ("a", FixtureEndpointKind::Boundary),
            ("b", FixtureEndpointKind::Finite),
        ],
        &[],
    ));
    assert_eq!(
        hex(&finite_a),
        "494e434400010012000000000000000200000000000000000161010000000000000001620000000000000000"
    );
    assert_eq!(
        hex(&boundary_a),
        "494e434400010012000000000000000201000000000000000161000000000000000001620000000000000000"
    );
    assert_ne!(finite_a, boundary_a);

    let forward = encoded(&topology(
        &[
            ("b", FixtureEndpointKind::Finite),
            ("a", FixtureEndpointKind::Finite),
        ],
        &[("a", "b")],
    ));
    let reverse = encoded(&topology(
        &[
            ("a", FixtureEndpointKind::Finite),
            ("b", FixtureEndpointKind::Finite),
        ],
        &[("b", "a")],
    ));
    assert_eq!(
        hex(&forward),
        "494e434400010012000000000000000200000000000000000161000000000000000001620000000000000001000000000000000161000000000000000162"
    );
    assert_eq!(
        hex(&reverse),
        "494e434400010012000000000000000200000000000000000161000000000000000001620000000000000001000000000000000162000000000000000161"
    );
    assert_ne!(forward, reverse);
}

#[test]
fn initial_stock_entry_presence_is_explicit_and_input_order_is_canonical() {
    let first_topology = topology(
        &[
            ("a", FixtureEndpointKind::Finite),
            ("b", FixtureEndpointKind::Boundary),
        ],
        &[("a", "b")],
    );
    let second_topology = topology(
        &[
            ("b", FixtureEndpointKind::Boundary),
            ("a", FixtureEndpointKind::Finite),
        ],
        &[("a", "b")],
    );
    let first_registry = registry(&["x"]);
    let second_registry = registry(&["x"]);
    let explicit = stocks(
        &first_topology,
        &first_registry,
        vec![(compartment("a"), sparse(&first_registry, &[("x", 0.0)]))],
    );
    let permuted = stocks(
        &second_topology,
        &second_registry,
        vec![(compartment("a"), sparse(&second_registry, &[]))],
    );
    let absent = stocks(&first_topology, &first_registry, vec![]);
    let explicit_golden = "494e4344000100130000000000000002000000000000000001610100000000000000016200000000000000010000000000000001610000000000000001620000000000000001000000000000000178000000000000000100000000000000016100000000000000010000000000000001780000000000000000";
    let absent_golden = "494e43440001001300000000000000020000000000000000016101000000000000000162000000000000000100000000000000016100000000000000016200000000000000010000000000000001780000000000000000";
    assert_eq!(hex(&encoded(&explicit)), explicit_golden);
    assert_eq!(encoded(&explicit), encoded(&permuted));
    assert_eq!(hex(&encoded(&absent)), absent_golden);
    assert_ne!(encoded(&explicit), encoded(&absent));
}

#[test]
fn calendar_and_horizon_fields_have_exact_distinct_encodings() {
    let base_calendar = encoded(&calendar(-1, 60));
    let moved_origin = encoded(&calendar(0, 60));
    let asymmetric_origin = encoded(&calendar(1, 60));
    let moved_duration = encoded(&calendar(-1, 61));
    assert_eq!(
        hex(&base_calendar),
        "494e434400010014ffffffffffffffff000000000000003c"
    );
    assert_eq!(
        hex(&moved_origin),
        "494e4344000100140000000000000000000000000000003c"
    );
    assert_eq!(
        hex(&asymmetric_origin),
        "494e4344000100140000000000000001000000000000003c"
    );
    assert_eq!(
        hex(&moved_duration),
        "494e434400010014ffffffffffffffff000000000000003d"
    );
    assert_ne!(base_calendar, moved_origin);
    assert_ne!(base_calendar, asymmetric_origin);
    assert_ne!(base_calendar, moved_duration);

    let base_horizon = encoded(&horizon(3, 5));
    let moved_last = encoded(&horizon(3, 6));
    let moved_first = encoded(&horizon(4, 5));
    assert_eq!(
        hex(&base_horizon),
        "494e43440001001500000000000000030000000000000005"
    );
    assert_eq!(
        hex(&moved_last),
        "494e43440001001500000000000000030000000000000006"
    );
    assert_eq!(
        hex(&moved_first),
        "494e43440001001500000000000000040000000000000005"
    );
    assert_ne!(base_horizon, moved_last);
    assert_ne!(base_horizon, moved_first);
}
