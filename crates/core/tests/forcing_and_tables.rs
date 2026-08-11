use incidence_core::canonical_encoding::CanonicalEncode;
use incidence_core::forcing::{ForcingSeries, ForcingSeriesError};
use incidence_core::interpolation_table::{
    InterpolationBoundaryPolicy, InterpolationTable, InterpolationTableError,
};
use incidence_core::numerical_semantics::{NumericalSemanticsVersion, ScalarComparison};
use incidence_core::presence::ValueState;
use incidence_core::rule_expression::{RuleExpr, RuleExprView};
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef, TableId,
};
use incidence_core::temporal::{RunHorizon, TimestepIndex};
use incidence_core::versions::{CanonicalEncodingVersion, RuleIrVersion};

fn forcing_id(value: &str) -> ForcingId {
    match ForcingId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("failed to parse forcing identity {value:?}: {error}"),
    }
}

fn table_id(value: &str) -> TableId {
    match TableId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("failed to parse table identity {value:?}: {error}"),
    }
}

fn input_ref(value: &str) -> InputRef {
    let id = match InputId::parse(value) {
        Ok(id) => id,
        Err(error) => panic!("failed to parse input identity {value:?}: {error}"),
    };
    InputRef::new(id, ExpressionValueKind::Scalar)
}

fn horizon(first: u64, last: u64) -> RunHorizon {
    match RunHorizon::new(TimestepIndex::new(first), TimestepIndex::new(last)) {
        Ok(horizon) => horizon,
        Err(error) => panic!("failed to construct horizon {first}..={last}: {error}"),
    }
}

fn forcing(first: u64, last: u64, values: Vec<f64>) -> ForcingSeries {
    match ForcingSeries::new(forcing_id("forcing-a"), horizon(first, last), values) {
        Ok(series) => series,
        Err(error) => panic!("failed to construct forcing series: {error}"),
    }
}

fn table(policy: InterpolationBoundaryPolicy, ordinates: Vec<f64>) -> InterpolationTable {
    match InterpolationTable::new(
        table_id("table-a"),
        NumericalSemanticsVersion::V1,
        policy,
        vec![-2.5, 1.25, 6.5],
        ordinates,
    ) {
        Ok(table) => table,
        Err(error) => panic!("failed to construct interpolation table: {error}"),
    }
}

fn encode<T: CanonicalEncode>(value: &T) -> Vec<u8> {
    match CanonicalEncodingVersion::V1.encode(value) {
        Ok(bytes) => bytes,
        Err(error) => panic!("failed to canonically encode value: {error}"),
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = match std::str::from_utf8(pair) {
                Ok(text) => text,
                Err(error) => panic!("invalid UTF-8 in hexadecimal fixture: {error}"),
            };
            match u8::from_str_radix(text, 16) {
                Ok(byte) => byte,
                Err(error) => panic!("invalid hexadecimal fixture byte {text:?}: {error}"),
            }
        })
        .collect()
}

#[test]
fn forcing_series_preserves_shape_presence_and_bits() {
    let series = forcing(3, 6, vec![1.25, 0.0, -3.5, 2.75]);
    assert_eq!(series.id().as_str(), "forcing-a");
    assert_eq!(series.horizon().first().value(), 3);
    assert_eq!(series.horizon().last().value(), 6);
    assert_eq!(series.len(), 4);
    assert!(!series.is_empty());
    assert_eq!(
        series.values().map(f64::to_bits).collect::<Vec<_>>(),
        [
            0x3ff4_0000_0000_0000,
            0,
            0xc00c_0000_0000_0000,
            0x4006_0000_0000_0000,
        ]
    );
    assert_eq!(
        series.value_at(TimestepIndex::new(3)),
        ValueState::Present(1.25)
    );
    assert_eq!(
        series.value_at(TimestepIndex::new(4)),
        ValueState::Present(0.0)
    );
    assert_eq!(
        series.value_at(TimestepIndex::new(5)),
        ValueState::Present(-3.5)
    );
    assert_eq!(
        series.value_at(TimestepIndex::new(6)),
        ValueState::Present(2.75)
    );
    assert_eq!(series.value_at(TimestepIndex::new(2)), ValueState::Absent);
    assert_eq!(series.value_at(TimestepIndex::new(7)), ValueState::Absent);
    match series.value_at(TimestepIndex::new(4)) {
        ValueState::Present(value) => assert_eq!(value.to_bits(), 0),
        ValueState::Absent => panic!("in-horizon explicit zero was absent"),
        ValueState::NotModelled => panic!("forcing series returned not-modelled"),
    }
    for timestep in 2..=7 {
        assert!(!matches!(
            series.value_at(TimestepIndex::new(timestep)),
            ValueState::NotModelled
        ));
    }
}

#[test]
fn forcing_series_rejects_invalid_counts_without_overflow() {
    for (values, actual) in [
        (vec![1.25, 0.0, -3.5], 3),
        (vec![1.25, 0.0, -3.5, 2.75, 8.0], 5),
    ] {
        assert_eq!(
            ForcingSeries::new(forcing_id("forcing-a"), horizon(3, 6), values),
            Err(ForcingSeriesError::ValueCountMismatch {
                first_timestep: 3,
                last_timestep: 6,
                expected: 4,
                actual,
            })
        );
    }
    assert_eq!(
        ForcingSeries::new(forcing_id("forcing-a"), horizon(0, u64::MAX), vec![]),
        Err(ForcingSeriesError::ValueCountMismatch {
            first_timestep: 0,
            last_timestep: u64::MAX,
            expected: 18_446_744_073_709_551_616_u128,
            actual: 0,
        })
    );
    let last = forcing(u64::MAX, u64::MAX, vec![1.25]);
    assert_eq!(
        last.value_at(TimestepIndex::new(u64::MAX)),
        ValueState::Present(1.25)
    );
}

#[test]
fn forcing_series_rejects_each_non_finite_value_at_its_timestep() {
    for (values, timestep, bits) in [
        (vec![1.25, f64::NAN, 2.75], 12, 0x7ff8_0000_0000_0000),
        (vec![f64::INFINITY, 0.0, 2.75], 11, 0x7ff0_0000_0000_0000),
        (
            vec![1.25, 0.0, f64::NEG_INFINITY],
            13,
            0xfff0_0000_0000_0000,
        ),
    ] {
        assert_eq!(
            ForcingSeries::new(forcing_id("forcing-a"), horizon(11, 13), values),
            Err(ForcingSeriesError::NonFiniteValue { timestep, bits })
        );
    }
}

#[test]
fn forcing_series_normalizes_zero_and_round_trips_exact_wire() {
    let negative = forcing(7, 7, vec![-0.0]);
    let positive = forcing(7, 7, vec![0.0]);
    assert_eq!(negative.values().map(f64::to_bits).collect::<Vec<_>>(), [0]);
    assert_eq!(
        negative.value_at(TimestepIndex::new(7)),
        ValueState::Present(0.0)
    );
    assert_eq!(
        serde_json::to_string(&negative).unwrap(),
        r#"{"id":"forcing-a","horizon":{"first":7,"last":7},"values":[0.0]}"#
    );
    assert_eq!(encode(&negative), encode(&positive));

    let series = forcing(3, 6, vec![1.25, 0.0, -3.5, 2.75]);
    let wire = r#"{"id":"forcing-a","horizon":{"first":3,"last":6},"values":[1.25,0.0,-3.5,2.75]}"#;
    assert_eq!(serde_json::to_string(&series).unwrap(), wire);
    let decoded: ForcingSeries = serde_json::from_str(wire).unwrap();
    assert_eq!(decoded, series);
    assert_eq!(serde_json::to_string(&decoded).unwrap(), wire);
    assert_eq!(encode(&decoded), encode(&series));
}

#[test]
fn forcing_series_serde_rejects_every_malformed_wire() {
    for wire in [
        r#"{"id":"forcing-a","horizon":{"first":6,"last":3},"values":[1.25,0.0,-3.5,2.75]}"#,
        r#"{"id":"forcing-a","horizon":{"first":3,"last":6},"values":[1.25,0.0,-3.5]}"#,
        r#"{"id":"Forcing-a","horizon":{"first":3,"last":6},"values":[1.25,0.0,-3.5,2.75]}"#,
        r#"{"id":"forcing-a","horizon":{"first":3,"last":6},"values":[1.25,0.0,-3.5,2.75],"opaque":true}"#,
        r#"{"id":"forcing-a","horizon":{"first":3,"last":6},"values":[1e400,0.0,-3.5,2.75]}"#,
    ] {
        assert!(
            serde_json::from_str::<ForcingSeries>(wire).is_err(),
            "accepted {wire}"
        );
    }
}

#[test]
fn forcing_series_canonical_encoding_is_exact_and_mutation_sensitive() {
    let base = forcing(3, 6, vec![1.25, 0.0, -3.5, 2.75]);
    let base_bytes = encode(&base);
    assert_eq!(
        base_bytes,
        decode_hex(
            "494e4344000100180000000000000009666f7263696e672d610000000000000003000000000000000600000000000000043ff40000000000000000000000000000c00c0000000000004006000000000000"
        )
    );

    let adjacent = f64::from_bits(0x3ff4_0000_0000_0001);
    let mutated = forcing(3, 6, vec![adjacent, 0.0, -3.5, 2.75]);
    assert_eq!(
        mutated.values().next().map(f64::to_bits),
        Some(0x3ff4_0000_0000_0001)
    );
    let mutated_bytes = encode(&mutated);
    assert_eq!(
        mutated_bytes,
        decode_hex(
            "494e4344000100180000000000000009666f7263696e672d610000000000000003000000000000000600000000000000043ff40000000000010000000000000000c00c0000000000004006000000000000"
        )
    );
    assert_ne!(mutated_bytes, base_bytes);
}

#[test]
fn interpolation_table_preserves_shape_order_and_bits() {
    let table = table(
        InterpolationBoundaryPolicy::ClampToEndpoint,
        vec![10.25, 3.5, -1.75],
    );
    assert_eq!(table.id().as_str(), "table-a");
    assert_eq!(
        table.numerical_semantics_version(),
        NumericalSemanticsVersion::V1
    );
    assert_eq!(
        table.boundary_policy(),
        InterpolationBoundaryPolicy::ClampToEndpoint
    );
    assert_eq!(table.len(), 3);
    assert!(!table.is_empty());
    assert_eq!(
        table
            .points()
            .map(|(x, y)| (x.to_bits(), y.to_bits()))
            .collect::<Vec<_>>(),
        [
            (0xc004_0000_0000_0000, 0x4024_8000_0000_0000),
            (0x3ff4_0000_0000_0000, 0x400c_0000_0000_0000),
            (0x401a_0000_0000_0000, 0xbffc_0000_0000_0000),
        ]
    );
}

#[test]
fn interpolation_table_rejects_invalid_shape_and_non_finite_coordinates() {
    assert_eq!(
        InterpolationTable::new(
            table_id("table-a"),
            NumericalSemanticsVersion::V1,
            InterpolationBoundaryPolicy::Reject,
            vec![-2.5, 1.25, 6.5],
            vec![10.25, 3.5]
        ),
        Err(InterpolationTableError::CoordinateCountMismatch {
            abscissae: 3,
            ordinates: 2
        })
    );
    assert_eq!(
        InterpolationTable::new(
            table_id("table-a"),
            NumericalSemanticsVersion::V1,
            InterpolationBoundaryPolicy::Reject,
            vec![-2.5],
            vec![10.25]
        ),
        Err(InterpolationTableError::TooFewPoints { points: 1 })
    );
    let finite_x = [-2.5, 1.25, 6.5];
    let finite_y = [10.25, 3.5, -1.75];
    for (index, value, bits) in [
        (0, f64::NAN, 0x7ff8_0000_0000_0000),
        (1, f64::INFINITY, 0x7ff0_0000_0000_0000),
        (2, f64::NEG_INFINITY, 0xfff0_0000_0000_0000),
    ] {
        let mut xs = finite_x.to_vec();
        xs[index] = value;
        assert_eq!(
            InterpolationTable::new(
                table_id("table-a"),
                NumericalSemanticsVersion::V1,
                InterpolationBoundaryPolicy::Reject,
                xs,
                finite_y.to_vec()
            ),
            Err(InterpolationTableError::NonFiniteAbscissa { index, bits })
        );
        let mut ys = finite_y.to_vec();
        ys[index] = value;
        assert_eq!(
            InterpolationTable::new(
                table_id("table-a"),
                NumericalSemanticsVersion::V1,
                InterpolationBoundaryPolicy::Reject,
                finite_x.to_vec(),
                ys
            ),
            Err(InterpolationTableError::NonFiniteOrdinate { index, bits })
        );
    }
}

#[test]
fn interpolation_table_rejects_first_non_increasing_pair_after_zero_normalization() {
    for (xs, expected) in [
        (
            vec![-2.5, 1.25, 1.25],
            InterpolationTableError::AbscissaeNotStrictlyIncreasing {
                left_index: 1,
                right_index: 2,
                left_bits: 0x3ff4_0000_0000_0000,
                right_bits: 0x3ff4_0000_0000_0000,
            },
        ),
        (
            vec![-2.5, 6.5, 1.25],
            InterpolationTableError::AbscissaeNotStrictlyIncreasing {
                left_index: 1,
                right_index: 2,
                left_bits: 0x401a_0000_0000_0000,
                right_bits: 0x3ff4_0000_0000_0000,
            },
        ),
        (
            vec![-0.0, 0.0],
            InterpolationTableError::AbscissaeNotStrictlyIncreasing {
                left_index: 0,
                right_index: 1,
                left_bits: 0,
                right_bits: 0,
            },
        ),
    ] {
        let ys = vec![0.0; xs.len()];
        assert_eq!(
            InterpolationTable::new(
                table_id("table-a"),
                NumericalSemanticsVersion::V1,
                InterpolationBoundaryPolicy::Reject,
                xs,
                ys
            ),
            Err(expected)
        );
    }
}

#[test]
fn interpolation_table_wire_and_policy_tokens_are_exact() {
    let table = table(
        InterpolationBoundaryPolicy::ClampToEndpoint,
        vec![10.25, 3.5, -1.75],
    );
    let wire = r#"{"id":"table-a","numerical_semantics_version":"v1","boundary_policy":"clamp_to_endpoint","abscissae":[-2.5,1.25,6.5],"ordinates":[10.25,3.5,-1.75]}"#;
    assert_eq!(serde_json::to_string(&table).unwrap(), wire);
    let decoded: InterpolationTable = serde_json::from_str(wire).unwrap();
    assert_eq!(decoded, table);
    assert_eq!(serde_json::to_string(&decoded).unwrap(), wire);
    assert_eq!(encode(&decoded), encode(&table));
    assert_eq!(
        serde_json::to_string(&InterpolationBoundaryPolicy::Reject).unwrap(),
        r#""reject""#
    );
    assert_eq!(
        serde_json::to_string(&InterpolationBoundaryPolicy::ClampToEndpoint).unwrap(),
        r#""clamp_to_endpoint""#
    );
    assert_eq!(
        serde_json::to_string(&InterpolationBoundaryPolicy::LinearExtrapolate).unwrap(),
        r#""linear_extrapolate""#
    );
}

#[test]
fn interpolation_table_serde_rejects_every_malformed_wire() {
    for wire in [
        r#"{"id":"table-a","numerical_semantics_version":"v1","boundary_policy":"reject","abscissae":[-2.5,1.25,6.5],"ordinates":[10.25,3.5]}"#,
        r#"{"id":"table-a","numerical_semantics_version":"v1","boundary_policy":"reject","abscissae":[-2.5],"ordinates":[10.25]}"#,
        r#"{"id":"table-a","numerical_semantics_version":"v1","boundary_policy":"reject","abscissae":[-2.5,1.25,1.25],"ordinates":[10.25,3.5,-1.75]}"#,
        r#"{"id":"table.a","numerical_semantics_version":"v1","boundary_policy":"reject","abscissae":[-2.5,1.25],"ordinates":[10.25,3.5]}"#,
        r#"{"id":"table-a","numerical_semantics_version":"v2","boundary_policy":"reject","abscissae":[-2.5,1.25],"ordinates":[10.25,3.5]}"#,
        r#"{"id":"table-a","numerical_semantics_version":"v1","boundary_policy":"nearest","abscissae":[-2.5,1.25],"ordinates":[10.25,3.5]}"#,
        r#"{"id":"table-a","numerical_semantics_version":"v1","boundary_policy":"reject","abscissae":[-2.5,1.25],"ordinates":[10.25,3.5],"opaque":true}"#,
        r#"{"id":"table-a","numerical_semantics_version":"v1","boundary_policy":"reject","abscissae":[-2.5,1e400],"ordinates":[10.25,3.5]}"#,
    ] {
        assert!(
            serde_json::from_str::<InterpolationTable>(wire).is_err(),
            "accepted {wire}"
        );
    }
}

#[test]
fn interpolation_table_canonical_encodings_are_exact_and_mutation_sensitive() {
    let fixtures = [
        (
            InterpolationBoundaryPolicy::Reject,
            0x00,
            "494e43440001001900000000000000077461626c652d610001000000000000000003c00400000000000040248000000000003ff4000000000000400c000000000000401a000000000000bffc000000000000",
        ),
        (
            InterpolationBoundaryPolicy::ClampToEndpoint,
            0x01,
            "494e43440001001900000000000000077461626c652d610001010000000000000003c00400000000000040248000000000003ff4000000000000400c000000000000401a000000000000bffc000000000000",
        ),
        (
            InterpolationBoundaryPolicy::LinearExtrapolate,
            0x02,
            "494e43440001001900000000000000077461626c652d610001020000000000000003c00400000000000040248000000000003ff4000000000000400c000000000000401a000000000000bffc000000000000",
        ),
    ];
    let mut encodings = Vec::new();
    for (policy, tag, golden) in fixtures {
        let bytes = encode(&table(policy, vec![10.25, 3.5, -1.75]));
        assert_eq!(bytes[25], tag);
        assert_eq!(bytes, decode_hex(golden));
        encodings.push(bytes);
    }
    assert_ne!(encodings[0], encodings[1]);
    assert_ne!(encodings[0], encodings[2]);
    assert_ne!(encodings[1], encodings[2]);

    let mutated = table(
        InterpolationBoundaryPolicy::ClampToEndpoint,
        vec![10.25, 3.5, f64::from_bits(0xbffc_0000_0000_0001)],
    );
    assert_eq!(
        mutated.points().last().map(|(_, y)| y.to_bits()),
        Some(0xbffc_0000_0000_0001)
    );
    let mutated_bytes = encode(&mutated);
    assert_eq!(
        mutated_bytes,
        decode_hex(
            "494e43440001001900000000000000077461626c652d610001010000000000000003c00400000000000040248000000000003ff4000000000000400c000000000000401a000000000000bffc000000000001"
        )
    );
    assert_ne!(mutated_bytes, encodings[1]);
}

#[test]
fn existing_forcing_and_table_identity_wires_remain_exact() {
    assert_eq!(
        serde_json::to_string(&forcing_id("forcing-a")).unwrap(),
        r#""forcing-a""#
    );
    assert_eq!(
        serde_json::to_string(&table_id("table-a")).unwrap(),
        r#""table-a""#
    );
    assert!(serde_json::from_str::<ForcingId>(r#""Forcing-a""#).is_err());
    assert!(serde_json::from_str::<TableId>(r#""table.a""#).is_err());
}

fn literal(value: f64) -> RuleExpr {
    match RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value) {
        Ok(expression) => expression,
        Err(error) => panic!("failed to construct literal: {error}"),
    }
}

fn evaporation_expression() -> RuleExpr {
    let stock = || {
        RuleExpr::input(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            input_ref("current-stock"),
        )
    };
    let condition = match RuleExpr::comparison(ScalarComparison::GreaterThan, stock(), literal(0.0))
    {
        Ok(expression) => expression,
        Err(error) => panic!("failed to construct comparison: {error}"),
    };
    let forcing = RuleExpr::forcing(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        ForcingRef::new(forcing_id("forcing-evaporation")),
    );
    let lookup = match RuleExpr::interpolated_table(
        InterpolatedTableRef::new(table_id("surface-area-by-stock")),
        stock(),
    ) {
        Ok(expression) => expression,
        Err(error) => panic!("failed to construct table expression: {error}"),
    };
    let when_true = match RuleExpr::multiply(forcing, lookup) {
        Ok(expression) => expression,
        Err(error) => panic!("failed to construct multiplication: {error}"),
    };
    match RuleExpr::select(condition, when_true, literal(0.0)) {
        Ok(expression) => expression,
        Err(error) => panic!("failed to construct selection: {error}"),
    }
}

fn assert_stock_input(expression: &RuleExpr) {
    match expression.view() {
        RuleExprView::Input(reference) => {
            assert_eq!(reference.id().as_str(), "current-stock");
            assert_eq!(reference.value_kind(), ExpressionValueKind::Scalar);
        }
        _ => panic!("expected current-stock scalar input"),
    }
}

fn assert_zero_literal(expression: &RuleExpr) {
    match expression.view() {
        RuleExprView::Literal(value) => assert_eq!(value.value().to_bits(), 0),
        _ => panic!("expected positive-zero literal"),
    }
}

#[test]
fn evaporation_shaped_fixture_uses_only_public_closed_ir() {
    let series = match ForcingSeries::new(
        forcing_id("forcing-evaporation"),
        horizon(11, 13),
        vec![0.125, 0.0, 1.75],
    ) {
        Ok(series) => series,
        Err(error) => panic!("failed to construct fixture series: {error}"),
    };
    let table = match InterpolationTable::new(
        table_id("surface-area-by-stock"),
        NumericalSemanticsVersion::V1,
        InterpolationBoundaryPolicy::ClampToEndpoint,
        vec![0.0, 7.25, 19.5],
        vec![0.0, 2.5, 8.75],
    ) {
        Ok(table) => table,
        Err(error) => panic!("failed to construct fixture table: {error}"),
    };
    let expression = evaporation_expression();
    assert_eq!(expression.value_kind(), ExpressionValueKind::Scalar);
    assert_eq!(expression.depth(), 4);

    match expression.view() {
        RuleExprView::Select {
            condition,
            when_true,
            when_false,
        } => {
            match condition.view() {
                RuleExprView::Comparison {
                    comparison,
                    lhs,
                    rhs,
                } => {
                    assert_eq!(comparison, ScalarComparison::GreaterThan);
                    assert_stock_input(lhs);
                    assert_zero_literal(rhs);
                }
                _ => panic!("expected greater-than condition"),
            }
            match when_true.view() {
                RuleExprView::Multiply { lhs, rhs } => {
                    match lhs.view() {
                        RuleExprView::Forcing(reference) => assert_eq!(reference.id(), series.id()),
                        _ => panic!("expected forcing on multiplication left"),
                    }
                    match rhs.view() {
                        RuleExprView::InterpolatedTable {
                            table: reference,
                            input,
                        } => {
                            assert_eq!(reference.id(), table.id());
                            assert_stock_input(input);
                        }
                        _ => panic!("expected table lookup on multiplication right"),
                    }
                }
                _ => panic!("expected multiplication on true branch"),
            }
            assert_zero_literal(when_false);
        }
        _ => panic!("expected outer selection"),
    }

    let forcing_wire = r#"{"id":"forcing-evaporation","horizon":{"first":11,"last":13},"values":[0.125,0.0,1.75]}"#;
    let table_wire = r#"{"id":"surface-area-by-stock","numerical_semantics_version":"v1","boundary_policy":"clamp_to_endpoint","abscissae":[0.0,7.25,19.5],"ordinates":[0.0,2.5,8.75]}"#;
    let expression_wire = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"select","condition":{"kind":"comparison","comparison":"greater_than","lhs":{"kind":"input","reference":{"id":"current-stock","value_kind":"scalar"}},"rhs":{"kind":"literal","value":0.0}},"when_true":{"kind":"multiply","lhs":{"kind":"forcing","reference":{"id":"forcing-evaporation"}},"rhs":{"kind":"interpolated_table","table":{"id":"surface-area-by-stock"},"input":{"kind":"input","reference":{"id":"current-stock","value_kind":"scalar"}}}},"when_false":{"kind":"literal","value":0.0}}}"#;
    assert_eq!(serde_json::to_string(&series).unwrap(), forcing_wire);
    assert_eq!(serde_json::to_string(&table).unwrap(), table_wire);
    assert_eq!(serde_json::to_string(&expression).unwrap(), expression_wire);
    let decoded: RuleExpr = serde_json::from_str(expression_wire).unwrap();
    assert_eq!(decoded, expression);
    assert_eq!(serde_json::to_string(&decoded).unwrap(), expression_wire);
    assert_eq!(encode(&decoded), encode(&expression));
}

#[test]
fn interpolation_table_normalizes_abscissae_before_ordinates() {
    assert_eq!(
        InterpolationTable::new(
            table_id("table-a"),
            NumericalSemanticsVersion::V1,
            InterpolationBoundaryPolicy::Reject,
            vec![f64::NAN, 1.25],
            vec![f64::NAN, 3.5]
        ),
        Err(InterpolationTableError::NonFiniteAbscissa {
            index: 0,
            bits: 0x7ff8_0000_0000_0000
        })
    );
}

#[test]
fn interpolation_table_reports_count_mismatch_before_too_few_points() {
    assert_eq!(
        InterpolationTable::new(
            table_id("table-a"),
            NumericalSemanticsVersion::V1,
            InterpolationBoundaryPolicy::Reject,
            vec![-2.5],
            vec![]
        ),
        Err(InterpolationTableError::CoordinateCountMismatch {
            abscissae: 1,
            ordinates: 0
        })
    );
}

#[test]
fn interpolation_table_normalizes_ordinates_before_checking_abscissa_order() {
    assert_eq!(
        InterpolationTable::new(
            table_id("table-a"),
            NumericalSemanticsVersion::V1,
            InterpolationBoundaryPolicy::Reject,
            vec![1.25, 1.25],
            vec![f64::NAN, 3.5]
        ),
        Err(InterpolationTableError::NonFiniteOrdinate {
            index: 0,
            bits: 0x7ff8_0000_0000_0000
        })
    );
}

#[test]
fn one_value_forcing_series_is_not_empty() {
    let series = forcing(7, 7, vec![-0.0]);
    assert_eq!(series.len(), 1);
    assert!(!series.is_empty());
}

#[test]
fn two_point_interpolation_table_is_not_empty() {
    match InterpolationTable::new(
        table_id("table-a"),
        NumericalSemanticsVersion::V1,
        InterpolationBoundaryPolicy::Reject,
        vec![-2.5, 1.25],
        vec![10.25, 3.5],
    ) {
        Ok(table) => {
            assert_eq!(table.len(), 2);
            assert!(!table.is_empty());
        }
        Err(error) => panic!("failed to construct two-point table: {error}"),
    }
}
