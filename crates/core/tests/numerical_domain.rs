//! Black-box acceptance tests for numerical domain invariants and V1 ordered accumulation.

use incidence_core::non_negative_amount::{NonNegativeAmount, NonNegativeAmountError};
use incidence_core::numerical_semantics::{
    AccumulationError, NumericalSemanticsError, NumericalSemanticsVersion, OperandPosition,
    ScalarComparison, ScalarOperation,
};
use incidence_core::signed_boundary_balance::{SignedBoundaryBalance, SignedBoundaryBalanceError};

fn parse_amounts(values: &[f64]) -> Vec<NonNegativeAmount> {
    values
        .iter()
        .map(|value| match NonNegativeAmount::try_from(*value) {
            Ok(amount) => amount,
            Err(error) => panic!("fixture must parse: {error}"),
        })
        .collect()
}

fn scalar(result: Result<f64, NumericalSemanticsError>) -> f64 {
    match result {
        Ok(value) => value,
        Err(error) => panic!("scalar fixture unexpectedly failed: {error}"),
    }
}

fn predicate(result: Result<bool, NumericalSemanticsError>) -> bool {
    match result {
        Ok(value) => value,
        Err(error) => panic!("predicate fixture unexpectedly failed: {error}"),
    }
}

fn assert_non_finite_operand<T>(
    result: Result<T, NumericalSemanticsError>,
    operation: ScalarOperation,
    position: OperandPosition,
    bits: u64,
) {
    match result {
        Err(NumericalSemanticsError::NonFiniteOperand {
            operation: actual_operation,
            position: actual_position,
            bits: actual_bits,
        }) => {
            assert_eq!(actual_operation, operation);
            assert_eq!(actual_position, position);
            assert_eq!(actual_bits, bits);
        }
        Ok(_) => panic!("non-finite operand unexpectedly succeeded"),
        Err(error) => panic!("wrong non-finite operand error: {error}"),
    }
}

#[test]
fn non_negative_amount_rejects_invalid_values_and_normalizes_zero() {
    let error = NonNegativeAmount::try_from(-1.0).expect_err("negative value must fail");
    assert_eq!(error, NonNegativeAmountError::Negative { value: -1.0 });
    let NonNegativeAmountError::Negative { value } = error else {
        panic!("expected negative error");
    };
    assert_eq!(value.to_bits(), 0xbff0000000000000);

    for input in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = NonNegativeAmount::try_from(input).expect_err("non-finite value must fail");
        let NonNegativeAmountError::NonFinite { value } = error else {
            panic!("expected non-finite error");
        };
        assert_eq!(value.to_bits(), input.to_bits());
    }

    let negative_zero = NonNegativeAmount::try_from(-0.0).expect("negative zero must parse");
    assert_eq!(negative_zero.value().to_bits(), 0x0000000000000000);
    let amount = NonNegativeAmount::try_from(4.25).expect("positive amount must parse");
    assert_eq!(amount.value().to_bits(), 0x4011000000000000);
    assert_eq!(
        NonNegativeAmount::ZERO.value().to_bits(),
        0x0000000000000000
    );
}

#[test]
fn signed_boundary_balance_accepts_both_signs_and_normalizes_zero() {
    let negative = SignedBoundaryBalance::try_from(-7.5).expect("negative balance must parse");
    assert_eq!(negative.value().to_bits(), 0xc01e000000000000);
    let positive = SignedBoundaryBalance::try_from(7.5).expect("positive balance must parse");
    assert_eq!(positive.value().to_bits(), 0x401e000000000000);
    let negative_zero = SignedBoundaryBalance::try_from(-0.0).expect("negative zero must parse");
    assert_eq!(negative_zero.value().to_bits(), 0x0000000000000000);
    assert_eq!(
        SignedBoundaryBalance::ZERO.value().to_bits(),
        0x0000000000000000
    );

    for input in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = SignedBoundaryBalance::try_from(input).expect_err("non-finite value must fail");
        let SignedBoundaryBalanceError::NonFinite { value } = error;
        assert_eq!(value.to_bits(), input.to_bits());
    }
}

#[test]
fn v1_accumulation_uses_authoritative_input_order() {
    let first = parse_amounts(&[10_000_000_000_000_000.0, 1.0, 1.0]);
    let first_result = NumericalSemanticsVersion::V1
        .accumulate(first)
        .expect("first fixture must accumulate");
    assert_eq!(first_result.value().to_bits(), 0x4341c37937e08000);

    let second = parse_amounts(&[1.0, 1.0, 10_000_000_000_000_000.0]);
    let second_result = NumericalSemanticsVersion::V1
        .accumulate(second)
        .expect("second fixture must accumulate");
    assert_eq!(second_result.value().to_bits(), 0x4341c37937e08001);
    assert_ne!(
        first_result.value().to_bits(),
        second_result.value().to_bits()
    );
}

#[test]
fn v1_accumulation_returns_canonical_positive_zero() {
    let empty = NumericalSemanticsVersion::V1
        .accumulate(std::iter::empty::<NonNegativeAmount>())
        .expect("empty fixture must accumulate");
    assert_eq!(empty.value().to_bits(), 0x0000000000000000);

    let zeros = parse_amounts(&[-0.0, 0.0]);
    let zero_result = NumericalSemanticsVersion::V1
        .accumulate(zeros)
        .expect("zero fixture must accumulate");
    assert_eq!(zero_result.value().to_bits(), 0x0000000000000000);
}

#[test]
fn v1_accumulation_fails_loudly_on_overflow() {
    let amounts = parse_amounts(&[f64::MAX, f64::MAX]);
    let error = NumericalSemanticsVersion::V1
        .accumulate(amounts)
        .expect_err("overflow must fail");

    let AccumulationError::NonFiniteSum {
        version,
        index,
        partial_sum,
        addend,
    } = error
    else {
        panic!("expected non-finite sum error");
    };
    assert_eq!(version, NumericalSemanticsVersion::V1);
    assert_eq!(index, 1);
    assert_eq!(partial_sum.to_bits(), 0x7fefffffffffffff);
    assert_eq!(addend.to_bits(), 0x7fefffffffffffff);
}

#[test]
fn v1_scalar_operations_canonicalize_zero_and_pin_ordinary_results() {
    let v1 = NumericalSemanticsVersion::V1;
    for zero in [-0.0, 0.0] {
        assert_eq!(scalar(v1.normalize(zero)).to_bits(), 0);
    }
    let zero_results = [
        scalar(v1.add(-0.0, 0.0)),
        scalar(v1.subtract(1.0, 1.0)),
        scalar(v1.multiply(-0.0, 2.0)),
        scalar(v1.divide(-0.0, 2.0)),
        scalar(v1.minimum(-0.0, 0.0)),
        scalar(v1.maximum(-0.0, 0.0)),
        scalar(v1.clamp(-0.0, -0.0, 0.0)),
        scalar(v1.select(1.0, -0.0, 2.0)),
        scalar(v1.select(0.0, 2.0, -0.0)),
    ];
    for result in zero_results {
        assert_eq!(result.to_bits(), 0x0000000000000000);
    }
    assert_eq!(scalar(v1.multiply(-1.0, 0.0)).to_bits(), 0x0000000000000000);
    assert_eq!(scalar(v1.divide(0.0, -1.0)).to_bits(), 0x0000000000000000);

    assert_eq!(scalar(v1.add(1.5, 2.25)).to_bits(), 0x400e000000000000);
    assert_eq!(scalar(v1.subtract(1.5, 2.25)).to_bits(), 0xbfe8000000000000);
    assert_eq!(scalar(v1.multiply(1.5, 2.0)).to_bits(), 0x4008000000000000);
    assert_eq!(scalar(v1.divide(3.0, 2.0)).to_bits(), 0x3ff8000000000000);
    assert_eq!(scalar(v1.minimum(2.0, 3.0)).to_bits(), 0x4000000000000000);
    assert_eq!(scalar(v1.maximum(2.0, 3.0)).to_bits(), 0x4008000000000000);
    assert_eq!(
        scalar(v1.clamp(4.0, 1.0, 3.0)).to_bits(),
        0x4008000000000000
    );
    assert_eq!(
        scalar(v1.clamp(0.0, 1.0, 3.0)).to_bits(),
        0x3ff0000000000000
    );
    assert_eq!(
        scalar(v1.clamp(2.0, 1.0, 3.0)).to_bits(),
        0x4000000000000000
    );
    assert_eq!(
        scalar(v1.clamp(2.0, 2.0, 2.0)).to_bits(),
        0x4000000000000000
    );
}

#[test]
fn v1_comparisons_truth_and_selection_are_exact() {
    let v1 = NumericalSemanticsVersion::V1;
    let comparisons = [
        (ScalarComparison::Equal, false),
        (ScalarComparison::NotEqual, true),
        (ScalarComparison::LessThan, true),
        (ScalarComparison::LessThanOrEqual, true),
        (ScalarComparison::GreaterThan, false),
        (ScalarComparison::GreaterThanOrEqual, false),
    ];
    for (comparison, expected) in comparisons {
        assert_eq!(predicate(v1.compare(comparison, 2.0, 3.0)), expected);
    }
    assert!(predicate(v1.compare(ScalarComparison::Equal, -0.0, 0.0)));
    assert!(predicate(v1.compare(
        ScalarComparison::LessThanOrEqual,
        -0.0,
        0.0
    )));
    assert!(predicate(v1.compare(
        ScalarComparison::GreaterThanOrEqual,
        -0.0,
        0.0
    )));
    assert!(!predicate(v1.compare(
        ScalarComparison::LessThan,
        -0.0,
        0.0
    )));
    assert!(!predicate(v1.compare(
        ScalarComparison::GreaterThan,
        -0.0,
        0.0
    )));

    assert!(!predicate(v1.truth(-0.0)));
    assert!(!predicate(v1.truth(0.0)));
    assert!(predicate(v1.truth(1.0)));
    assert!(predicate(v1.truth(-1.0)));
    assert_eq!(
        scalar(v1.select(0.0, 11.0, 22.0)).to_bits(),
        0x4036000000000000
    );
    assert_eq!(
        scalar(v1.select(-1.0, 11.0, 22.0)).to_bits(),
        0x4026000000000000
    );
}

#[test]
fn every_scalar_entry_point_rejects_each_non_finite_operand_position() {
    let v1 = NumericalSemanticsVersion::V1;
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let bits = invalid.to_bits();
        assert_non_finite_operand(
            v1.normalize(invalid),
            ScalarOperation::Normalize,
            OperandPosition::Value,
            bits,
        );
        for (result, operation) in [
            (v1.add(invalid, 1.0), ScalarOperation::Add),
            (v1.subtract(invalid, 1.0), ScalarOperation::Subtract),
            (v1.multiply(invalid, 1.0), ScalarOperation::Multiply),
            (v1.divide(invalid, 1.0), ScalarOperation::Divide),
            (v1.minimum(invalid, 1.0), ScalarOperation::Minimum),
            (v1.maximum(invalid, 1.0), ScalarOperation::Maximum),
        ] {
            assert_non_finite_operand(result, operation, OperandPosition::Lhs, bits);
        }
        for (result, operation) in [
            (v1.add(1.0, invalid), ScalarOperation::Add),
            (v1.subtract(1.0, invalid), ScalarOperation::Subtract),
            (v1.multiply(1.0, invalid), ScalarOperation::Multiply),
            (v1.divide(1.0, invalid), ScalarOperation::Divide),
            (v1.minimum(1.0, invalid), ScalarOperation::Minimum),
            (v1.maximum(1.0, invalid), ScalarOperation::Maximum),
        ] {
            assert_non_finite_operand(result, operation, OperandPosition::Rhs, bits);
        }
        assert_non_finite_operand(
            v1.compare(ScalarComparison::Equal, invalid, 1.0),
            ScalarOperation::Compare,
            OperandPosition::Lhs,
            bits,
        );
        assert_non_finite_operand(
            v1.compare(ScalarComparison::Equal, 1.0, invalid),
            ScalarOperation::Compare,
            OperandPosition::Rhs,
            bits,
        );
        assert_non_finite_operand(
            v1.clamp(invalid, 0.0, 2.0),
            ScalarOperation::Clamp,
            OperandPosition::Value,
            bits,
        );
        assert_non_finite_operand(
            v1.clamp(1.0, invalid, 2.0),
            ScalarOperation::Clamp,
            OperandPosition::Lower,
            bits,
        );
        assert_non_finite_operand(
            v1.clamp(1.0, 0.0, invalid),
            ScalarOperation::Clamp,
            OperandPosition::Upper,
            bits,
        );
        assert_non_finite_operand(
            v1.truth(invalid),
            ScalarOperation::Truth,
            OperandPosition::Value,
            bits,
        );
        assert_non_finite_operand(
            v1.select(invalid, 1.0, 2.0),
            ScalarOperation::Select,
            OperandPosition::Condition,
            bits,
        );
        assert_non_finite_operand(
            v1.select(1.0, invalid, 2.0),
            ScalarOperation::Select,
            OperandPosition::WhenTrue,
            bits,
        );
        assert_non_finite_operand(
            v1.select(1.0, 1.0, invalid),
            ScalarOperation::Select,
            OperandPosition::WhenFalse,
            bits,
        );
        for (result, position) in [
            (
                v1.interpolate_linear(invalid, 0.0, 2.0, 3.0, 4.0),
                OperandPosition::X,
            ),
            (
                v1.interpolate_linear(1.0, invalid, 2.0, 3.0, 4.0),
                OperandPosition::X0,
            ),
            (
                v1.interpolate_linear(1.0, 0.0, invalid, 3.0, 4.0),
                OperandPosition::X1,
            ),
            (
                v1.interpolate_linear(1.0, 0.0, 2.0, invalid, 4.0),
                OperandPosition::Y0,
            ),
            (
                v1.interpolate_linear(1.0, 0.0, 2.0, 3.0, invalid),
                OperandPosition::Y1,
            ),
        ] {
            assert_non_finite_operand(result, ScalarOperation::InterpolateLinear, position, bits);
        }
    }
}

#[test]
fn v1_arithmetic_rejects_non_finite_results_with_operation_metadata() {
    let v1 = NumericalSemanticsVersion::V1;
    for (result, operation) in [
        (v1.add(f64::MAX, f64::MAX), ScalarOperation::Add),
        (v1.subtract(-f64::MAX, f64::MAX), ScalarOperation::Subtract),
        (v1.multiply(f64::MAX, 2.0), ScalarOperation::Multiply),
        (v1.divide(1.0, 0.0), ScalarOperation::Divide),
    ] {
        let bits = match operation {
            ScalarOperation::Subtract => 0xfff0000000000000,
            _ => 0x7ff0000000000000,
        };
        assert_eq!(
            result,
            Err(NumericalSemanticsError::NonFiniteResult { operation, bits })
        );
    }
    match v1.divide(0.0, 0.0) {
        Err(NumericalSemanticsError::NonFiniteResult {
            operation: ScalarOperation::Divide,
            bits,
        }) => assert!(f64::from_bits(bits).is_nan()),
        Ok(value) => panic!("zero divided by zero unexpectedly produced {value}"),
        Err(error) => panic!("wrong zero-divide error: {error}"),
    }
}

#[test]
fn v1_rejects_invalid_bounds_with_exact_canonical_bits() {
    let v1 = NumericalSemanticsVersion::V1;
    assert_eq!(
        v1.clamp(2.0, 3.0, 1.0),
        Err(NumericalSemanticsError::InvalidClampBounds {
            lower_bits: 0x4008000000000000,
            upper_bits: 0x3ff0000000000000,
        })
    );
    assert_eq!(
        v1.interpolate_linear(1.0, 1.0, 1.0, 2.0, 3.0),
        Err(NumericalSemanticsError::InvalidInterpolationAbscissae {
            x0_bits: 0x3ff0000000000000,
            x1_bits: 0x3ff0000000000000,
        })
    );
    assert_eq!(
        v1.interpolate_linear(1.0, 2.0, 1.0, 2.0, 3.0),
        Err(NumericalSemanticsError::InvalidInterpolationAbscissae {
            x0_bits: 0x4000000000000000,
            x1_bits: 0x3ff0000000000000,
        })
    );
}

#[test]
fn v1_linear_interpolation_pins_six_operation_order() {
    let result = scalar(NumericalSemanticsVersion::V1.interpolate_linear(
        3.0,
        0.0,
        10.0,
        10_000_000_000_000_000.0,
        10_000_000_000_000_004.0,
    ));
    assert_eq!(result.to_bits(), 0x4341c37937e08001);

    let fraction: f64 = 3.0 / 10.0;
    let left_weight = 1.0 - fraction;
    let weighted_left = left_weight * 10_000_000_000_000_000.0;
    let weighted_right = fraction * 10_000_000_000_000_004.0;
    let forbidden = weighted_left + weighted_right;
    assert_eq!(forbidden.to_bits(), 0x4341c37937e08000);
    assert_ne!(result.to_bits(), forbidden.to_bits());
}
