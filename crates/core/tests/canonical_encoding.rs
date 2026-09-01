//! Black-box golden tests for canonical V1 bytes.

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::numerical_semantics::ScalarComparison;
use incidence_core::partition_expression::{Fraction, FractionBranch, PartitionExpr};
use incidence_core::projection::{
    AuthoritativeFactSelector, BoundedLagSpec, FiniteRecurrenceSpec, InitialProjectionValue,
    InitialProjectorState, ProjectionSet, ProjectionSource, ProjectionSpec, RecurrenceInputBinding,
    RecurrenceInputSource,
};
use incidence_core::rule_expression::{RuleExpr, RuleExprError};
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef,
    ParameterId, ParameterRef, ProjectionId, ProjectionRef, ProjectionValueKind, TableId,
    TransferBranchId,
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
            ProjectionValueKind::Scalar,
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
    for (kind, expected_tag) in [
        (ProjectionValueKind::Scalar, 0),
        (ProjectionValueKind::Truth, 1),
        (ProjectionValueKind::Extensive, 2),
    ] {
        let projection = RuleExpr::projection(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            ProjectionRef::new(rule_projection_id("projection-kind-a"), kind),
        );
        assert_eq!(encoded(&projection)[13], expected_tag);
    }
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

fn mixed_recurrence_spec() -> ProjectionSpec {
    let scalar_input = RuleExpr::input(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        InputRef::new(rule_input_id("scalar-a"), ExpressionValueKind::Scalar),
    );
    let truth_input = RuleExpr::input(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        InputRef::new(rule_input_id("truth-a"), ExpressionValueKind::Truth),
    );
    let truth_parameter = RuleExpr::parameter(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        ParameterRef::new(rule_parameter_id("truth-p"), ExpressionValueKind::Truth),
    );
    let scalar_parameter = RuleExpr::parameter(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        ParameterRef::new(rule_parameter_id("scalar-p"), ExpressionValueKind::Scalar),
    );
    let scalar_update = match RuleExpr::select(truth_parameter, scalar_input, scalar_parameter) {
        Ok(value) => value,
        Err(error) => panic!("mixed select fixture must construct: {error}"),
    };
    match FiniteRecurrenceSpec::new(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        rule_projection_id("rec-mixed"),
        vec![ProjectionValueKind::Scalar, ProjectionValueKind::Truth],
        vec![
            RecurrenceInputBinding::new(
                InputRef::new(rule_input_id("scalar-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::PreviousState {
                    index: 0,
                    value_kind: ProjectionValueKind::Scalar,
                },
            ),
            RecurrenceInputBinding::new(
                InputRef::new(rule_input_id("truth-a"), ExpressionValueKind::Truth),
                RecurrenceInputSource::PreviousState {
                    index: 1,
                    value_kind: ProjectionValueKind::Truth,
                },
            ),
        ],
        vec![
            ParameterRef::new(rule_parameter_id("truth-p"), ExpressionValueKind::Truth),
            ParameterRef::new(rule_parameter_id("scalar-p"), ExpressionValueKind::Scalar),
        ],
        vec![scalar_update, truth_input],
        0,
    ) {
        Ok(value) => value.into(),
        Err(error) => panic!("mixed recurrence fixture must construct: {error}"),
    }
}

fn fixture_advance(cursor: &mut usize, amount: usize) {
    *cursor = match cursor.checked_add(amount) {
        Some(value) => value,
        None => panic!("fixture cursor overflow"),
    };
}

fn fixture_u64(bytes: &[u8], cursor: &mut usize) -> usize {
    let end = match cursor.checked_add(8) {
        Some(value) => value,
        None => panic!("fixture count cursor overflow"),
    };
    let raw: [u8; 8] = match bytes[*cursor..end].try_into() {
        Ok(value) => value,
        Err(error) => panic!("fixture count must occupy eight bytes: {error}"),
    };
    *cursor = end;
    match usize::try_from(u64::from_be_bytes(raw)) {
        Ok(value) => value,
        Err(error) => panic!("fixture count must fit usize: {error}"),
    }
}

fn fixture_string<'a>(bytes: &'a [u8], cursor: &mut usize) -> &'a str {
    let length = fixture_u64(bytes, cursor);
    let end = match cursor.checked_add(length) {
        Some(value) => value,
        None => panic!("fixture string cursor overflow"),
    };
    let value = match std::str::from_utf8(&bytes[*cursor..end]) {
        Ok(value) => value,
        Err(error) => panic!("fixture identity must be UTF-8: {error}"),
    };
    *cursor = end;
    value
}

fn recurrence_tag_positions<'a>(
    bytes: &'a [u8],
) -> (Vec<usize>, Vec<usize>, Vec<usize>, Vec<&'a str>) {
    let mut cursor = 8;
    fixture_advance(&mut cursor, 4);
    let _ = fixture_string(bytes, &mut cursor);
    fixture_advance(&mut cursor, 2);
    let state_count = fixture_u64(bytes, &mut cursor);
    let mut state_positions = Vec::new();
    for _ in 0..state_count {
        state_positions.push(cursor);
        fixture_advance(&mut cursor, 1);
    }
    let input_count = fixture_u64(bytes, &mut cursor);
    let mut previous_positions = Vec::new();
    for _ in 0..input_count {
        fixture_advance(&mut cursor, 1);
        let _ = fixture_string(bytes, &mut cursor);
        let source_tag = bytes[cursor];
        fixture_advance(&mut cursor, 1);
        match source_tag {
            0 => {
                fixture_advance(&mut cursor, 1);
                let _ = fixture_string(bytes, &mut cursor);
                let _ = fixture_string(bytes, &mut cursor);
            }
            1 => {
                fixture_advance(&mut cursor, 8);
                previous_positions.push(cursor);
                fixture_advance(&mut cursor, 1);
            }
            other => panic!("unexpected recurrence input source tag {other}"),
        }
    }
    let parameter_count = fixture_u64(bytes, &mut cursor);
    let mut parameter_positions = Vec::new();
    let mut parameter_ids = Vec::new();
    for _ in 0..parameter_count {
        parameter_positions.push(cursor);
        fixture_advance(&mut cursor, 1);
        parameter_ids.push(fixture_string(bytes, &mut cursor));
    }
    (
        state_positions,
        previous_positions,
        parameter_positions,
        parameter_ids,
    )
}

fn initial_value_positions(bytes: &[u8]) -> (Vec<usize>, Vec<usize>) {
    let mut cursor = 8;
    let _ = fixture_string(bytes, &mut cursor);
    let count = fixture_u64(bytes, &mut cursor);
    let mut kind_positions = Vec::new();
    let mut payload_positions = Vec::new();
    for _ in 0..count {
        kind_positions.push(cursor);
        let kind = bytes[cursor];
        fixture_advance(&mut cursor, 1);
        payload_positions.push(cursor);
        match kind {
            0 | 1 => fixture_advance(&mut cursor, 8),
            2 => fixture_advance(&mut cursor, 1),
            other => panic!("unexpected initial value kind tag {other}"),
        }
    }
    (kind_positions, payload_positions)
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

#[test]
fn projection_specifications_and_states_have_exact_v1_encodings() {
    const SPEC_JSON: [&str; 3] = [
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"lag-a","value_kind":"extensive","spec":{"kind":"bounded_lag","source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}},"steps":3}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"rolling-a","value_kind":"extensive","spec":{"kind":"ordered_rolling_aggregate","source":{"kind":"projection","reference":{"id":"lag-a","value_kind":"extensive"}},"window":5,"aggregate":"sum_oldest_to_newest"}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"rec-a","value_kind":"extensive","spec":{"kind":"finite_recurrence","state_kinds":["extensive"],"inputs":[{"reference":{"id":"current-a","value_kind":"scalar"},"source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}}},{"reference":{"id":"previous-a","value_kind":"scalar"},"source":{"kind":"previous_state","index":0,"value_kind":"extensive"}}],"parameters":[{"id":"coefficient-a","value_kind":"scalar"}],"updates":[{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"add","lhs":{"kind":"add","lhs":{"kind":"input","reference":{"id":"current-a","value_kind":"scalar"}},"rhs":{"kind":"input","reference":{"id":"previous-a","value_kind":"scalar"}}},"rhs":{"kind":"parameter","reference":{"id":"coefficient-a","value_kind":"scalar"}}}}],"output_index":0}}"#,
    ];
    const SPEC_GOLDEN: [&str; 3] = [
        "494e4344000100180001000100000000000000056c61672d6100000000000000000000000772656163682d61000000000000000b7375627374616e63652d610000000000000003",
        "494e434400010018000100010000000000000009726f6c6c696e672d610001010000000000000000056c61672d61000000000000000500",
        "494e4344000100180001000100000000000000057265632d610002000000000000000100000000000000000200000000000000000963757272656e742d610000000000000000000772656163682d61000000000000000b7375627374616e63652d6100000000000000000a70726576696f75732d6101000000000000000000000000000000000100000000000000000d636f656666696369656e742d6100000000000000010001000105050000000000000000000963757272656e742d610000000000000000000a70726576696f75732d610100000000000000000d636f656666696369656e742d610000000000000000",
    ];
    const STATE_JSON: [&str; 3] = [
        r#"{"projection":"lag-a","values":[{"kind":"extensive","value":1.25},{"kind":"extensive","value":2.5},{"kind":"extensive","value":3.75}]}"#,
        r#"{"projection":"rolling-a","values":[{"kind":"extensive","value":0.625},{"kind":"extensive","value":1.75},{"kind":"extensive","value":2.875},{"kind":"extensive","value":4.125}]}"#,
        r#"{"projection":"rec-a","values":[{"kind":"extensive","value":6.75}]}"#,
    ];
    const STATE_GOLDEN: [&str; 3] = [
        "494e43440001001900000000000000056c61672d610000000000000003003ff400000000000000400400000000000000400e000000000000",
        "494e4344000100190000000000000009726f6c6c696e672d610000000000000004003fe4000000000000003ffc000000000000004007000000000000004010800000000000",
        "494e43440001001900000000000000057265632d61000000000000000100401b000000000000",
    ];
    let specs: Vec<ProjectionSpec> = SPEC_JSON
        .iter()
        .map(|json| match serde_json::from_str(json) {
            Ok(value) => value,
            Err(error) => panic!("projection JSON must decode: {error}"),
        })
        .collect();
    let states: Vec<InitialProjectorState> = STATE_JSON
        .iter()
        .map(|json| match serde_json::from_str(json) {
            Ok(value) => value,
            Err(error) => panic!("state JSON must decode: {error}"),
        })
        .collect();
    let recurrence_selector = AuthoritativeFactSelector::IncomingTransferAmount {
        compartment: match CompartmentId::parse("reach-a") {
            Ok(value) => value,
            Err(error) => panic!("recurrence compartment fixture failed: {error}"),
        },
        substance: match SubstanceId::parse("substance-a") {
            Ok(value) => value,
            Err(error) => panic!("recurrence substance fixture failed: {error}"),
        },
    };
    let recurrence_update = match RuleExpr::add(
        match RuleExpr::add(
            rule_scalar_input("current-a"),
            rule_scalar_input("previous-a"),
        ) {
            Ok(value) => value,
            Err(error) => panic!("inner recurrence add failed: {error}"),
        },
        RuleExpr::parameter(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            ParameterRef::new(
                rule_parameter_id("coefficient-a"),
                ExpressionValueKind::Scalar,
            ),
        ),
    ) {
        Ok(value) => value,
        Err(error) => panic!("outer recurrence add failed: {error}"),
    };
    let reversed_recurrence: ProjectionSpec = match FiniteRecurrenceSpec::new(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        rule_projection_id("rec-a"),
        vec![ProjectionValueKind::Extensive],
        vec![
            RecurrenceInputBinding::new(
                InputRef::new(rule_input_id("previous-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::PreviousState {
                    index: 0,
                    value_kind: ProjectionValueKind::Extensive,
                },
            ),
            RecurrenceInputBinding::new(
                InputRef::new(rule_input_id("current-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(recurrence_selector),
            ),
        ],
        vec![ParameterRef::new(
            rule_parameter_id("coefficient-a"),
            ExpressionValueKind::Scalar,
        )],
        vec![recurrence_update],
        0,
    ) {
        Ok(value) => value.into(),
        Err(error) => panic!("reversed recurrence fixture failed: {error}"),
    };
    assert_eq!(reversed_recurrence, specs[2]);
    let reversed_recurrence_bytes = encoded(&reversed_recurrence);
    assert_eq!(hex(&reversed_recurrence_bytes), SPEC_GOLDEN[2]);
    let (state_positions, previous_positions, _, _) =
        recurrence_tag_positions(&reversed_recurrence_bytes);
    assert_eq!(
        state_positions
            .iter()
            .map(|position| reversed_recurrence_bytes[*position])
            .collect::<Vec<_>>(),
        [0]
    );
    assert_eq!(
        previous_positions
            .iter()
            .map(|position| reversed_recurrence_bytes[*position])
            .collect::<Vec<_>>(),
        [0]
    );
    assert_eq!(
        match serde_json::to_string(&reversed_recurrence) {
            Ok(value) => value,
            Err(error) => panic!("reversed recurrence must serialize: {error}"),
        },
        SPEC_JSON[2]
    );
    for ((spec, expected), json) in specs.iter().zip(SPEC_GOLDEN).zip(SPEC_JSON) {
        let bytes = encoded(spec);
        assert_eq!(&bytes[6..8], &[0, 0x18]);
        assert_eq!(hex(&bytes), expected);
        let serialized = match serde_json::to_string(spec) {
            Ok(value) => value,
            Err(error) => panic!("projection must serialize: {error}"),
        };
        assert_eq!(serialized, json);
    }
    for ((state, expected), json) in states.iter().zip(STATE_GOLDEN).zip(STATE_JSON) {
        let bytes = encoded(state);
        assert_eq!(&bytes[6..8], &[0, 0x19]);
        assert_eq!(hex(&bytes), expected);
        let serialized = match serde_json::to_string(state) {
            Ok(value) => value,
            Err(error) => panic!("state must serialize: {error}"),
        };
        assert_eq!(serialized, json);
    }
    for left in 0..specs.len() {
        for right in (left + 1)..specs.len() {
            assert_ne!(encoded(&specs[left]), encoded(&specs[right]));
        }
    }
    for left in 0..states.len() {
        for right in (left + 1)..states.len() {
            assert_ne!(encoded(&states[left]), encoded(&states[right]));
        }
    }

    for (spec, expected_family_tag) in specs.iter().zip([0u8, 1, 2]) {
        let bytes = encoded(spec);
        let identity_length = u64::from_be_bytes(match bytes[12..20].try_into() {
            Ok(value) => value,
            Err(error) => panic!("identity length bytes must have length eight: {error}"),
        });
        let identity_length = match usize::try_from(identity_length) {
            Ok(value) => value,
            Err(error) => panic!("fixture identity length must fit usize: {error}"),
        };
        assert_eq!(bytes[21 + identity_length], expected_family_tag);
    }
}

#[test]
fn projection_kind_tags_are_pinned_extensive_zero_scalar_one_truth_two() {
    for (kind, tag) in [
        (ProjectionValueKind::Extensive, 0u8),
        (ProjectionValueKind::Scalar, 1u8),
        (ProjectionValueKind::Truth, 2u8),
    ] {
        let spec: ProjectionSpec = match BoundedLagSpec::new(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            rule_projection_id("lag-b"),
            ProjectionSource::Projection(ProjectionRef::new(rule_projection_id("lag-a"), kind)),
            1,
        ) {
            Ok(value) => value.into(),
            Err(error) => panic!("lag fixture must construct: {error}"),
        };
        let bytes = encoded(&spec);
        let identity_len = usize::from(bytes[19]);
        assert_eq!(
            bytes[20 + identity_len],
            tag,
            "output kind tag for {kind:?}"
        );
        assert_eq!(
            bytes[20 + identity_len + 3],
            tag,
            "source reference kind tag for {kind:?}"
        );
    }
}

#[test]
fn recurrence_output_index_changes_canonical_bytes_in_a_two_slot_state() {
    let selector = || AuthoritativeFactSelector::IncomingTransferAmount {
        compartment: match CompartmentId::parse("reach-a") {
            Ok(value) => value,
            Err(error) => panic!("compartment fixture: {error}"),
        },
        substance: match SubstanceId::parse("substance-a") {
            Ok(value) => value,
            Err(error) => panic!("substance fixture: {error}"),
        },
    };
    let build = |output_index: usize| -> ProjectionSpec {
        let leaf = rule_scalar_input("input-a");
        match FiniteRecurrenceSpec::new(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            rule_projection_id("rec-b"),
            vec![
                ProjectionValueKind::Extensive,
                ProjectionValueKind::Extensive,
            ],
            vec![RecurrenceInputBinding::new(
                InputRef::new(rule_input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(selector()),
            )],
            vec![],
            vec![leaf.clone(), leaf],
            output_index,
        ) {
            Ok(value) => value.into(),
            Err(error) => panic!("recurrence fixture must construct: {error}"),
        }
    };
    let zero = encoded(&build(0));
    let one = encoded(&build(1));
    assert_eq!(&zero[..zero.len() - 8], &one[..one.len() - 8]);
    assert_eq!(&zero[zero.len() - 8..], &[0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&one[one.len() - 8..], &[0, 0, 0, 0, 0, 0, 0, 1]);
}

#[test]
fn projection_canonical_encoding_discriminates_every_planned_field_and_normalizes_order() {
    const LAG: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"lag-a","value_kind":"extensive","spec":{"kind":"bounded_lag","source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}},"steps":3}}"#;
    const ROLLING: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"rolling-a","value_kind":"extensive","spec":{"kind":"ordered_rolling_aggregate","source":{"kind":"projection","reference":{"id":"lag-a","value_kind":"extensive"}},"window":5,"aggregate":"sum_oldest_to_newest"}}"#;
    const RECURRENCE: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"rec-a","value_kind":"extensive","spec":{"kind":"finite_recurrence","state_kinds":["extensive"],"inputs":[{"reference":{"id":"current-a","value_kind":"scalar"},"source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}}},{"reference":{"id":"previous-a","value_kind":"scalar"},"source":{"kind":"previous_state","index":0,"value_kind":"extensive"}}],"parameters":[{"id":"coefficient-a","value_kind":"scalar"}],"updates":[{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"add","lhs":{"kind":"add","lhs":{"kind":"input","reference":{"id":"current-a","value_kind":"scalar"}},"rhs":{"kind":"input","reference":{"id":"previous-a","value_kind":"scalar"}}},"rhs":{"kind":"parameter","reference":{"id":"coefficient-a","value_kind":"scalar"}}}}],"output_index":0}}"#;

    let decode = |json: &str| -> ProjectionSpec {
        match serde_json::from_str(json) {
            Ok(value) => value,
            Err(error) => panic!("projection mutation must decode: {error}\n{json}"),
        }
    };
    let differs = |base: &str, mutated: String| {
        assert_ne!(encoded(&decode(base)), encoded(&decode(&mutated)));
    };

    differs(LAG, LAG.replace("reach-a", "reach-b"));
    differs(LAG, LAG.replace("substance-a", "substance-b"));
    differs(
        LAG,
        LAG.replace("incoming_transfer_amount", "outgoing_transfer_amount"),
    );
    differs(LAG, LAG.replace(r#""steps":3"#, r#""steps":4"#));
    differs(ROLLING, ROLLING.replace(r#""window":5"#, r#""window":6"#));
    differs(ROLLING, ROLLING.replace("lag-a", "lag-b"));
    differs(
        RECURRENCE,
        RECURRENCE
            .replace(
                r#""id":"rec-a","value_kind":"extensive""#,
                r#""id":"rec-a","value_kind":"scalar""#,
            )
            .replace(
                r#""state_kinds":["extensive"]"#,
                r#""state_kinds":["scalar"]"#,
            )
            .replace(
                r#""kind":"previous_state","index":0,"value_kind":"extensive""#,
                r#""kind":"previous_state","index":0,"value_kind":"scalar""#,
            ),
    );

    const CURRENT: &str =
        r#"{"kind":"input","reference":{"id":"current-a","value_kind":"scalar"}}"#;
    const PREVIOUS: &str =
        r#"{"kind":"input","reference":{"id":"previous-a","value_kind":"scalar"}}"#;
    let inner = format!(r#""lhs":{CURRENT},"rhs":{PREVIOUS}"#);
    let reversed_inner = format!(r#""lhs":{PREVIOUS},"rhs":{CURRENT}"#);
    differs(RECURRENCE, RECURRENCE.replace(&inner, &reversed_inner));
    differs(
        RECURRENCE,
        RECURRENCE.replace("coefficient-a", "coefficient-b"),
    );

    let nested = format!(r#"{{"kind":"add","lhs":{CURRENT},"rhs":{PREVIOUS}}}"#);
    let parameter =
        r#"{"kind":"parameter","reference":{"id":"coefficient-a","value_kind":"scalar"}}"#;
    let outer = format!(r#""lhs":{nested},"rhs":{parameter}"#);
    let reversed_outer = format!(r#""lhs":{parameter},"rhs":{nested}"#);
    differs(RECURRENCE, RECURRENCE.replace(&outer, &reversed_outer));

    let make_recurrence = |reverse: bool| -> ProjectionSpec {
        let selector = AuthoritativeFactSelector::IncomingTransferAmount {
            compartment: match CompartmentId::parse("reach-a") {
                Ok(value) => value,
                Err(error) => panic!("compartment fixture: {error}"),
            },
            substance: match SubstanceId::parse("substance-a") {
                Ok(value) => value,
                Err(error) => panic!("substance fixture: {error}"),
            },
        };
        let current = RecurrenceInputBinding::new(
            InputRef::new(rule_input_id("current-a"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::AuthoritativeFact(selector),
        );
        let previous = RecurrenceInputBinding::new(
            InputRef::new(rule_input_id("previous-a"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::PreviousState {
                index: 0,
                value_kind: ProjectionValueKind::Extensive,
            },
        );
        let coefficient_a = ParameterRef::new(
            rule_parameter_id("coefficient-a"),
            ExpressionValueKind::Scalar,
        );
        let coefficient_b = ParameterRef::new(
            rule_parameter_id("coefficient-b"),
            ExpressionValueKind::Scalar,
        );
        let add = |lhs, rhs| match RuleExpr::add(lhs, rhs) {
            Ok(value) => value,
            Err(error) => panic!("add fixture must construct: {error}"),
        };
        let update = add(
            add(
                rule_scalar_input("current-a"),
                rule_scalar_input("previous-a"),
            ),
            add(
                RuleExpr::parameter(
                    RuleIrVersion::V1,
                    NumericalSemanticsVersion::V1,
                    coefficient_a.clone(),
                ),
                RuleExpr::parameter(
                    RuleIrVersion::V1,
                    NumericalSemanticsVersion::V1,
                    coefficient_b.clone(),
                ),
            ),
        );
        let (inputs, parameters) = if reverse {
            (vec![previous, current], vec![coefficient_b, coefficient_a])
        } else {
            (vec![current, previous], vec![coefficient_a, coefficient_b])
        };
        match FiniteRecurrenceSpec::new(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            rule_projection_id("rec-order"),
            vec![ProjectionValueKind::Extensive],
            inputs,
            parameters,
            vec![update],
            0,
        ) {
            Ok(value) => value.into(),
            Err(error) => panic!("ordered recurrence fixture must construct: {error}"),
        }
    };
    let authored = make_recurrence(false);
    let reversed = make_recurrence(true);
    assert_eq!(encoded(&authored), encoded(&reversed));
    assert_eq!(
        match serde_json::to_string(&authored) {
            Ok(value) => value,
            Err(error) => panic!("authored recurrence must serialize: {error}"),
        },
        match serde_json::to_string(&reversed) {
            Ok(value) => value,
            Err(error) => panic!("reversed recurrence must serialize: {error}"),
        }
    );

    for (state_json, old, new) in [
        (
            r#"{"projection":"lag-a","values":[{"kind":"extensive","value":1.25},{"kind":"extensive","value":2.5},{"kind":"extensive","value":3.75}]}"#,
            "1.25",
            "1.375",
        ),
        (
            r#"{"projection":"lag-a","values":[{"kind":"extensive","value":1.25},{"kind":"extensive","value":2.5},{"kind":"extensive","value":3.75}]}"#,
            "2.5",
            "2.625",
        ),
        (
            r#"{"projection":"lag-a","values":[{"kind":"extensive","value":1.25},{"kind":"extensive","value":2.5},{"kind":"extensive","value":3.75}]}"#,
            "3.75",
            "3.875",
        ),
        (
            r#"{"projection":"rolling-a","values":[{"kind":"extensive","value":0.625},{"kind":"extensive","value":1.75},{"kind":"extensive","value":2.875},{"kind":"extensive","value":4.125}]}"#,
            "0.625",
            "0.75",
        ),
        (
            r#"{"projection":"rolling-a","values":[{"kind":"extensive","value":0.625},{"kind":"extensive","value":1.75},{"kind":"extensive","value":2.875},{"kind":"extensive","value":4.125}]}"#,
            "1.75",
            "1.875",
        ),
        (
            r#"{"projection":"rolling-a","values":[{"kind":"extensive","value":0.625},{"kind":"extensive","value":1.75},{"kind":"extensive","value":2.875},{"kind":"extensive","value":4.125}]}"#,
            "2.875",
            "3.0",
        ),
        (
            r#"{"projection":"rolling-a","values":[{"kind":"extensive","value":0.625},{"kind":"extensive","value":1.75},{"kind":"extensive","value":2.875},{"kind":"extensive","value":4.125}]}"#,
            "4.125",
            "4.25",
        ),
        (
            r#"{"projection":"rec-a","values":[{"kind":"extensive","value":6.75}]}"#,
            "6.75",
            "6.875",
        ),
    ] {
        let base: InitialProjectorState = match serde_json::from_str(state_json) {
            Ok(value) => value,
            Err(error) => panic!("base state must decode: {error}"),
        };
        let mutated_json = state_json.replacen(old, new, 1);
        let mutated: InitialProjectorState = match serde_json::from_str(&mutated_json) {
            Ok(value) => value,
            Err(error) => panic!("mutated state must decode: {error}"),
        };
        assert_ne!(encoded(&base), encoded(&mutated));
    }
}

#[test]
fn recurrence_state_and_previous_state_kind_tags_are_pinned_at_every_position() {
    const JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"rec-mixed","value_kind":"scalar","spec":{"kind":"finite_recurrence","state_kinds":["scalar","truth"],"inputs":[{"reference":{"id":"scalar-a","value_kind":"scalar"},"source":{"kind":"previous_state","index":0,"value_kind":"scalar"}},{"reference":{"id":"truth-a","value_kind":"truth"},"source":{"kind":"previous_state","index":1,"value_kind":"truth"}}],"parameters":[{"id":"scalar-p","value_kind":"scalar"},{"id":"truth-p","value_kind":"truth"}],"updates":[{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"select","condition":{"kind":"parameter","reference":{"id":"truth-p","value_kind":"truth"}},"when_true":{"kind":"input","reference":{"id":"scalar-a","value_kind":"scalar"}},"when_false":{"kind":"parameter","reference":{"id":"scalar-p","value_kind":"scalar"}}}},{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"input","reference":{"id":"truth-a","value_kind":"truth"}}}],"output_index":0}}"#;
    const GOLDEN: &str = "494e4344000100180001000100000000000000097265632d6d6978656401020000000000000002010200000000000000020000000000000000087363616c61722d610100000000000000000101000000000000000774727574682d610100000000000000010200000000000000020000000000000000087363616c61722d7001000000000000000774727574682d700000000000000002000100010d0101000000000000000774727574682d70000000000000000000087363616c61722d61010000000000000000087363616c61722d70000100010001000000000000000774727574682d610000000000000000";
    let spec = mixed_recurrence_spec();
    let bytes = encoded(&spec);
    assert_eq!(hex(&bytes), GOLDEN);
    assert_eq!(&bytes[6..8], &[0, 0x18]);
    let (state_positions, previous_positions, parameter_positions, parameter_ids) =
        recurrence_tag_positions(&bytes);
    assert_eq!(state_positions, [39, 40]);
    assert_eq!(previous_positions, [75, 101]);
    assert_eq!(parameter_positions, [110, 127]);
    assert_eq!(parameter_ids, ["scalar-p", "truth-p"]);
    assert_eq!(
        state_positions
            .iter()
            .map(|position| bytes[*position])
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(
        previous_positions
            .iter()
            .map(|position| bytes[*position])
            .collect::<Vec<_>>(),
        [1, 2]
    );
    let serialized = match serde_json::to_string(&spec) {
        Ok(value) => value,
        Err(error) => panic!("mixed recurrence must serialize: {error}"),
    };
    assert_eq!(serialized, JSON);
    let decoded: ProjectionSpec = match serde_json::from_str(JSON) {
        Ok(value) => value,
        Err(error) => panic!("mixed recurrence must decode: {error}"),
    };
    assert_eq!(decoded, spec);
    assert_eq!(encoded(&decoded), bytes);
}

#[test]
fn selector_direction_tags_discriminate_incoming_from_outgoing() {
    const INCOMING_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"lag-a","value_kind":"extensive","spec":{"kind":"bounded_lag","source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}},"steps":3}}"#;
    const OUTGOING_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"lag-a","value_kind":"extensive","spec":{"kind":"bounded_lag","source":{"kind":"authoritative_fact","selector":{"kind":"outgoing_transfer_amount","compartment":"reach-a","substance":"substance-a"}},"steps":3}}"#;
    const INCOMING_GOLDEN: &str = "494e4344000100180001000100000000000000056c61672d6100000000000000000000000772656163682d61000000000000000b7375627374616e63652d610000000000000003";
    const OUTGOING_GOLDEN: &str = "494e4344000100180001000100000000000000056c61672d6100000001000000000000000772656163682d61000000000000000b7375627374616e63652d610000000000000003";
    let incoming: ProjectionSpec = match serde_json::from_str(INCOMING_JSON) {
        Ok(value) => value,
        Err(error) => panic!("incoming lag must decode: {error}"),
    };
    let outgoing: ProjectionSpec = match serde_json::from_str(OUTGOING_JSON) {
        Ok(value) => value,
        Err(error) => panic!("outgoing lag must decode: {error}"),
    };
    let incoming_bytes = encoded(&incoming);
    let outgoing_bytes = encoded(&outgoing);
    assert_eq!(&incoming_bytes[6..8], &[0, 0x18]);
    assert_eq!(&outgoing_bytes[6..8], &[0, 0x18]);
    assert_eq!(hex(&incoming_bytes), INCOMING_GOLDEN);
    assert_eq!(hex(&outgoing_bytes), OUTGOING_GOLDEN);
    let differing = incoming_bytes
        .iter()
        .zip(&outgoing_bytes)
        .enumerate()
        .filter_map(|(index, (left, right))| (left != right).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(differing.len(), 1);
    assert_eq!(incoming_bytes[differing[0]], 0);
    assert_eq!(outgoing_bytes[differing[0]], 1);
    assert_eq!(
        INCOMING_JSON.replace("incoming_transfer_amount", "outgoing_transfer_amount"),
        OUTGOING_JSON
    );

    let advisory = INCOMING_JSON.replace(r#""value_kind":"extensive""#, r#""value_kind":"truth""#);
    let error = serde_json::from_str::<ProjectionSpec>(&advisory)
        .expect_err("a declared projection value kind must match the derived kind");
    assert!(
        error
            .to_string()
            .contains("does not match derived Extensive")
    );
}

#[test]
fn recurrence_input_selector_direction_tags_discriminate_incoming_from_outgoing() {
    const INCOMING: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"rec-a","value_kind":"extensive","spec":{"kind":"finite_recurrence","state_kinds":["extensive"],"inputs":[{"reference":{"id":"current-a","value_kind":"scalar"},"source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}}},{"reference":{"id":"previous-a","value_kind":"scalar"},"source":{"kind":"previous_state","index":0,"value_kind":"extensive"}}],"parameters":[{"id":"coefficient-a","value_kind":"scalar"}],"updates":[{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"add","lhs":{"kind":"add","lhs":{"kind":"input","reference":{"id":"current-a","value_kind":"scalar"}},"rhs":{"kind":"input","reference":{"id":"previous-a","value_kind":"scalar"}}},"rhs":{"kind":"parameter","reference":{"id":"coefficient-a","value_kind":"scalar"}}}}],"output_index":0}}"#;
    const INCOMING_GOLDEN: &str = "494e4344000100180001000100000000000000057265632d610002000000000000000100000000000000000200000000000000000963757272656e742d610000000000000000000772656163682d61000000000000000b7375627374616e63652d6100000000000000000a70726576696f75732d6101000000000000000000000000000000000100000000000000000d636f656666696369656e742d6100000000000000010001000105050000000000000000000963757272656e742d610000000000000000000a70726576696f75732d610100000000000000000d636f656666696369656e742d610000000000000000";
    const OUTGOING_GOLDEN: &str = "494e4344000100180001000100000000000000057265632d610002000000000000000100000000000000000200000000000000000963757272656e742d610001000000000000000772656163682d61000000000000000b7375627374616e63652d6100000000000000000a70726576696f75732d6101000000000000000000000000000000000100000000000000000d636f656666696369656e742d6100000000000000010001000105050000000000000000000963757272656e742d610000000000000000000a70726576696f75732d610100000000000000000d636f656666696369656e742d610000000000000000";
    let outgoing_json = INCOMING.replace("incoming_transfer_amount", "outgoing_transfer_amount");
    let incoming: ProjectionSpec = match serde_json::from_str(INCOMING) {
        Ok(value) => value,
        Err(error) => panic!("incoming recurrence must decode: {error}"),
    };
    let outgoing: ProjectionSpec = match serde_json::from_str(&outgoing_json) {
        Ok(value) => value,
        Err(error) => panic!("outgoing recurrence must decode: {error}"),
    };
    let incoming_bytes = encoded(&incoming);
    let outgoing_bytes = encoded(&outgoing);
    assert_eq!(&incoming_bytes[6..8], &[0, 0x18]);
    assert_eq!(&outgoing_bytes[6..8], &[0, 0x18]);
    assert_eq!(hex(&incoming_bytes), INCOMING_GOLDEN);
    assert_eq!(hex(&outgoing_bytes), OUTGOING_GOLDEN);
    let differing = incoming_bytes
        .iter()
        .zip(&outgoing_bytes)
        .enumerate()
        .filter_map(|(index, (left, right))| (left != right).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(differing, [63]);
    assert_eq!(incoming_bytes[63], 0);
    assert_eq!(outgoing_bytes[63], 1);
}

#[test]
fn initial_state_kind_and_truth_payload_tags_are_exact() {
    const TRUE_JSON: &str = r#"{"projection":"rec-mixed","values":[{"kind":"scalar","value":11.25},{"kind":"truth","value":true}]}"#;
    const FALSE_JSON: &str = r#"{"projection":"rec-mixed","values":[{"kind":"scalar","value":11.25},{"kind":"truth","value":false}]}"#;
    const TRUE_GOLDEN: &str =
        "494e43440001001900000000000000097265632d6d6978656400000000000000020140268000000000000201";
    const FALSE_GOLDEN: &str =
        "494e43440001001900000000000000097265632d6d6978656400000000000000020140268000000000000200";
    for (json, golden, payload) in [(TRUE_JSON, TRUE_GOLDEN, 1u8), (FALSE_JSON, FALSE_GOLDEN, 0)] {
        let state: InitialProjectorState = match serde_json::from_str(json) {
            Ok(value) => value,
            Err(error) => panic!("mixed state must decode: {error}"),
        };
        let bytes = encoded(&state);
        assert_eq!(hex(&bytes), golden);
        assert_eq!(&bytes[6..8], &[0, 0x19]);
        let (kind_positions, payload_positions) = initial_value_positions(&bytes);
        assert_eq!(kind_positions, [33, 42]);
        assert_eq!(payload_positions, [34, 43]);
        assert_eq!(
            kind_positions
                .iter()
                .map(|position| bytes[*position])
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(bytes[payload_positions[1]], payload);
        assert_eq!(
            match serde_json::to_string(&state) {
                Ok(value) => value,
                Err(error) => panic!("mixed state must serialize: {error}"),
            },
            json
        );
        assert!(ProjectionSet::new(vec![mixed_recurrence_spec()], vec![state]).is_ok());
    }
    let true_state: InitialProjectorState = match serde_json::from_str(TRUE_JSON) {
        Ok(value) => value,
        Err(error) => panic!("true state must decode: {error}"),
    };
    let false_state: InitialProjectorState = match serde_json::from_str(FALSE_JSON) {
        Ok(value) => value,
        Err(error) => panic!("false state must decode: {error}"),
    };
    let true_bytes = encoded(&true_state);
    let false_bytes = encoded(&false_state);
    assert_eq!(
        &true_bytes[..true_bytes.len() - 1],
        &false_bytes[..false_bytes.len() - 1]
    );
    assert_eq!(
        [
            true_bytes[true_bytes.len() - 1],
            false_bytes[false_bytes.len() - 1]
        ],
        [1, 0]
    );

    let extensive = match InitialProjectorState::new(
        rule_projection_id("lag-a"),
        NumericalSemanticsVersion::V1,
        vec![InitialProjectionValue::Extensive(-0.0)],
    ) {
        Ok(value) => value,
        Err(error) => panic!("zero state must construct: {error}"),
    };
    assert_eq!(extensive.values()[0].bits(), Some(0));
    assert_eq!(
        match serde_json::to_string(&extensive) {
            Ok(value) => value,
            Err(error) => panic!("zero state must serialize: {error}"),
        },
        r#"{"projection":"lag-a","values":[{"kind":"extensive","value":0.0}]}"#
    );
    let extensive_bytes = encoded(&extensive);
    let (extensive_kind_positions, _) = initial_value_positions(&extensive_bytes);
    assert_eq!(extensive_kind_positions.len(), 1);
    assert_eq!(extensive_bytes[extensive_kind_positions[0]], 0);
    assert_eq!(
        hex(&extensive_bytes),
        "494e43440001001900000000000000056c61672d610000000000000001000000000000000000"
    );
}

#[test]
fn parameter_declaration_order_and_kind_tags_are_pinned() {
    let spec = mixed_recurrence_spec();
    let bytes = encoded(&spec);
    let (_, _, positions, ids) = recurrence_tag_positions(&bytes);
    assert_eq!(positions, [110, 127]);
    assert_eq!(ids, ["scalar-p", "truth-p"]);
    assert_eq!(
        positions
            .iter()
            .map(|position| bytes[*position])
            .collect::<Vec<_>>(),
        [0, 1]
    );
    let json = match serde_json::to_string(&spec) {
        Ok(value) => value,
        Err(error) => panic!("mixed recurrence must serialize: {error}"),
    };
    let scalar = match json.find(r#"{"id":"scalar-p","value_kind":"scalar"}"#) {
        Some(value) => value,
        None => panic!("scalar parameter declaration missing"),
    };
    let truth = match json.find(r#"{"id":"truth-p","value_kind":"truth"}"#) {
        Some(value) => value,
        None => panic!("truth parameter declaration missing"),
    };
    assert!(scalar < truth);
}
