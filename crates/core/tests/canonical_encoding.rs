//! Black-box golden tests for canonical V1 bytes.

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::numerical_semantics::ScalarComparison;
use incidence_core::partition_expression::{Fraction, FractionBranch, PartitionExpr};
use incidence_core::rule_expression::{RuleExpr, RuleExprError};
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef,
    ParameterId, ParameterRef, ProjectionId, ProjectionRef, TableId, TransferBranchId,
};
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

#[test]
fn rule_expression_variants_have_exact_v1_encodings_and_tags() {
    let input = rule_scalar_input("input-a");
    let parameter = RuleExpr::parameter(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        ParameterRef::new(
            rule_parameter_id("parameter-a"),
            ExpressionValueKind::Scalar,
        ),
    );
    let forcing = RuleExpr::forcing(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        ForcingRef::new(rule_forcing_id("forcing-a")),
    );
    let projection = RuleExpr::projection(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        ProjectionRef::new(
            rule_projection_id("projection-a"),
            ExpressionValueKind::Scalar,
        ),
    );
    let literal = rule_literal(1.25);
    let add = rule_binary(RuleExpr::add, 1.25, 2.5);
    let subtract = rule_binary(RuleExpr::subtract, 1.25, 2.5);
    let multiply = rule_binary(RuleExpr::multiply, 1.25, 2.5);
    let divide = rule_binary(RuleExpr::divide, 1.25, 2.5);
    let minimum = rule_binary(RuleExpr::minimum, 1.25, 2.5);
    let maximum = rule_binary(RuleExpr::maximum, 1.25, 2.5);
    let clamp = match RuleExpr::clamp(rule_literal(2.5), rule_literal(1.25), rule_literal(3.75)) {
        Ok(v) => v,
        Err(e) => panic!("invalid clamp fixture: {e}"),
    };
    let comparison = match RuleExpr::comparison(
        ScalarComparison::GreaterThan,
        rule_literal(1.25),
        rule_literal(2.5),
    ) {
        Ok(v) => v,
        Err(e) => panic!("invalid comparison fixture: {e}"),
    };
    let select = match RuleExpr::select(
        rule_truth_input("truth-a"),
        rule_literal(1.25),
        rule_literal(2.5),
    ) {
        Ok(v) => v,
        Err(e) => panic!("invalid select fixture: {e}"),
    };
    let table = match RuleExpr::interpolated_table(
        InterpolatedTableRef::new(rule_table_id("table-a")),
        rule_literal(1.25),
    ) {
        Ok(v) => v,
        Err(e) => panic!("invalid table fixture: {e}"),
    };
    let values = [
        input, parameter, forcing, projection, literal, add, subtract, multiply, divide, minimum,
        maximum, clamp, comparison, select, table,
    ];
    let expected = [
        "494e4344000100160001000100000000000000000007696e7075742d61",
        "494e434400010016000100010100000000000000000b706172616d657465722d61",
        "494e43440001001600010001020000000000000009666f7263696e672d61",
        "494e434400010016000100010300000000000000000c70726f6a656374696f6e2d61",
        "494e43440001001600010001043ff4000000000000",
        "494e4344000100160001000105043ff4000000000000044004000000000000",
        "494e4344000100160001000106043ff4000000000000044004000000000000",
        "494e4344000100160001000107043ff4000000000000044004000000000000",
        "494e4344000100160001000108043ff4000000000000044004000000000000",
        "494e4344000100160001000109043ff4000000000000044004000000000000",
        "494e434400010016000100010a043ff4000000000000044004000000000000",
        "494e434400010016000100010b044004000000000000043ff400000000000004400e000000000000",
        "494e434400010016000100010c04043ff4000000000000044004000000000000",
        "494e434400010016000100010d0001000000000000000774727574682d61043ff4000000000000044004000000000000",
        "494e434400010016000100010e00000000000000077461626c652d61043ff4000000000000",
    ];
    let encodings: Vec<Vec<u8>> = values.iter().map(encoded).collect();
    for (bytes, expected) in encodings.iter().zip(expected) {
        assert_eq!(hex(bytes), expected);
    }
    assert_eq!(
        encodings.iter().map(|bytes| bytes[12]).collect::<Vec<_>>(),
        (0_u8..=14).collect::<Vec<_>>()
    );
    for left in 0..encodings.len() {
        for right in left + 1..encodings.len() {
            assert_ne!(encodings[left], encodings[right]);
        }
    }
    assert_eq!(&encodings[0][8..10], &[0, 1]);
    assert_eq!(&encodings[0][10..12], &[0, 1]);
    for value in values {
        let json = match serde_json::to_string(&value) {
            Ok(v) => v,
            Err(e) => panic!("rule serialization failed: {e}"),
        };
        let decoded: RuleExpr = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("rule deserialization failed: {e}"),
        };
        assert_eq!(decoded, value);
    }
}

#[test]
fn rule_expression_discriminating_fields_change_exact_bytes() {
    let ordered = encoded(&rule_binary(RuleExpr::subtract, 1.25, 2.5));
    let reversed = encoded(&rule_binary(RuleExpr::subtract, 2.5, 1.25));
    assert_eq!(
        hex(&ordered),
        "494e4344000100160001000106043ff4000000000000044004000000000000"
    );
    assert_eq!(
        hex(&reversed),
        "494e4344000100160001000106044004000000000000043ff4000000000000"
    );
    assert_ne!(ordered, reversed);
    let comparisons = [
        ScalarComparison::Equal,
        ScalarComparison::NotEqual,
        ScalarComparison::LessThan,
        ScalarComparison::LessThanOrEqual,
        ScalarComparison::GreaterThan,
        ScalarComparison::GreaterThanOrEqual,
    ];
    for (tag, comparison) in comparisons.into_iter().enumerate() {
        let value = match RuleExpr::comparison(comparison, rule_literal(1.25), rule_literal(2.5)) {
            Ok(v) => v,
            Err(e) => panic!("comparison failed: {e}"),
        };
        let expected_tag = match u8::try_from(tag) {
            Ok(value) => value,
            Err(error) => panic!("comparison tag must fit u8: {error}"),
        };
        assert_eq!(encoded(&value)[13], expected_tag);
    }
    let scalar = rule_scalar_input("input-a");
    let truth = rule_truth_input("input-a");
    let scalar_bytes = encoded(&scalar);
    let truth_bytes = encoded(&truth);
    assert_eq!(scalar_bytes[13], 0);
    assert_eq!(truth_bytes[13], 1);
    assert_ne!(scalar_bytes, truth_bytes);
    let scalar_json = match serde_json::to_string(&scalar) {
        Ok(v) => v,
        Err(e) => panic!("scalar input failed: {e}"),
    };
    let truth_json = match serde_json::to_string(&truth) {
        Ok(v) => v,
        Err(e) => panic!("truth input failed: {e}"),
    };
    let scalar_round: RuleExpr = match serde_json::from_str(&scalar_json) {
        Ok(v) => v,
        Err(e) => panic!("scalar round trip failed: {e}"),
    };
    let truth_round: RuleExpr = match serde_json::from_str(&truth_json) {
        Ok(v) => v,
        Err(e) => panic!("truth round trip failed: {e}"),
    };
    assert_eq!(scalar_round.value_kind(), ExpressionValueKind::Scalar);
    assert_eq!(truth_round.value_kind(), ExpressionValueKind::Truth);
    assert_ne!(encoded(&rule_literal(1.25)), encoded(&rule_literal(1.5)));
    for (value, bits) in [
        (f64::NAN, 0x7ff8_0000_0000_0000),
        (f64::INFINITY, 0x7ff0_0000_0000_0000),
        (f64::NEG_INFINITY, 0xfff0_0000_0000_0000),
    ] {
        assert_eq!(
            RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value),
            Err(RuleExprError::NonFiniteLiteral { bits })
        );
    }
    let negative_zero = rule_literal(-0.0);
    let positive_zero = rule_literal(0.0);
    assert_eq!(encoded(&negative_zero), encoded(&positive_zero));
}

#[test]
fn partition_variants_have_exact_v1_encodings_and_tags() {
    let values = [
        PartitionExpr::retain_all(RuleIrVersion::V1, NumericalSemanticsVersion::V1),
        PartitionExpr::release_all(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            transfer_branch_id("branch-a"),
        ),
        fixed_split(vec![("branch-a", 0.375), ("branch-b", 0.5)]),
        PartitionExpr::exogenous_series(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            transfer_branch_id("branch-a"),
            ForcingRef::new(rule_forcing_id("forcing-a")),
        ),
        match PartitionExpr::constant_fraction_transfer(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            transfer_branch_id("branch-a"),
            canonical_fraction(0.375),
        ) {
            Ok(v) => v,
            Err(e) => panic!("constant split failed: {e}"),
        },
    ];
    let expected = [
        "494e4344000100170001000100",
        "494e434400010017000100010100000000000000086272616e63682d61",
        "494e43440001001700010001023fc0000000000000000000000000000200000000000000086272616e63682d613fd800000000000000000000000000086272616e63682d623fe0000000000000",
        "494e434400010017000100010300000000000000086272616e63682d610000000000000009666f7263696e672d61",
        "494e434400010017000100010400000000000000086272616e63682d613fd8000000000000",
    ];
    let encodings: Vec<Vec<u8>> = values.iter().map(encoded).collect();
    for (bytes, expected) in encodings.iter().zip(expected) {
        assert_eq!(hex(bytes), expected);
    }
    assert_eq!(
        encodings.iter().map(|bytes| bytes[12]).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    for left in 0..encodings.len() {
        for right in left + 1..encodings.len() {
            assert_ne!(encodings[left], encodings[right]);
        }
    }
    assert_eq!(&encodings[0][8..10], &[0, 1]);
    assert_eq!(&encodings[0][10..12], &[0, 1]);
    assert_eq!(
        encoded(&fixed_split(vec![("branch-b", 0.5), ("branch-a", 0.375)])),
        encodings[2]
    );
    let changed_branch = PartitionExpr::release_all(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        transfer_branch_id("branch-b"),
    );
    assert_ne!(encoded(&changed_branch), encodings[1]);
    let changed_forcing = PartitionExpr::exogenous_series(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        transfer_branch_id("branch-a"),
        ForcingRef::new(rule_forcing_id("forcing-b")),
    );
    assert_ne!(encoded(&changed_forcing), encodings[3]);
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

fn rule_input_id(value: &str) -> InputId {
    match InputId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid rule input fixture: {error}"),
    }
}
fn rule_parameter_id(value: &str) -> ParameterId {
    match ParameterId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid rule parameter fixture: {error}"),
    }
}
fn rule_forcing_id(value: &str) -> ForcingId {
    match ForcingId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid rule forcing fixture: {error}"),
    }
}
fn rule_projection_id(value: &str) -> ProjectionId {
    match ProjectionId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid projection fixture: {error}"),
    }
}
fn rule_table_id(value: &str) -> TableId {
    match TableId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid table fixture: {error}"),
    }
}
fn transfer_branch_id(value: &str) -> TransferBranchId {
    match TransferBranchId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("invalid transfer branch fixture: {error}"),
    }
}
fn rule_literal(value: f64) -> RuleExpr {
    match RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value) {
        Ok(expr) => expr,
        Err(error) => panic!("invalid rule literal fixture: {error}"),
    }
}
fn rule_scalar_input(value: &str) -> RuleExpr {
    RuleExpr::input(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        InputRef::new(rule_input_id(value), ExpressionValueKind::Scalar),
    )
}
fn rule_truth_input(value: &str) -> RuleExpr {
    RuleExpr::input(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        InputRef::new(rule_input_id(value), ExpressionValueKind::Truth),
    )
}
fn rule_binary(
    constructor: fn(RuleExpr, RuleExpr) -> Result<RuleExpr, RuleExprError>,
    lhs: f64,
    rhs: f64,
) -> RuleExpr {
    match constructor(rule_literal(lhs), rule_literal(rhs)) {
        Ok(expr) => expr,
        Err(error) => panic!("invalid binary rule fixture: {error}"),
    }
}
fn canonical_fraction(value: f64) -> Fraction {
    match Fraction::new(NumericalSemanticsVersion::V1, value) {
        Ok(value) => value,
        Err(error) => panic!("invalid fraction fixture: {error}"),
    }
}
fn fixed_split(branches: Vec<(&str, f64)>) -> PartitionExpr {
    let branches = branches
        .into_iter()
        .map(|(id, value)| FractionBranch::new(transfer_branch_id(id), canonical_fraction(value)))
        .collect();
    match PartitionExpr::fixed_fraction_split(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        canonical_fraction(0.125),
        branches,
    ) {
        Ok(value) => value,
        Err(error) => panic!("invalid split fixture: {error}"),
    }
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
