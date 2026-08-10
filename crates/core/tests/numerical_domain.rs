//! Black-box acceptance tests for numerical domain invariants and V1 ordered accumulation.

use myproject_core::non_negative_amount::{NonNegativeAmount, NonNegativeAmountError};
use myproject_core::numerical_semantics::{AccumulationError, NumericalSemanticsVersion};
use myproject_core::signed_boundary_balance::{SignedBoundaryBalance, SignedBoundaryBalanceError};

fn parse_amounts(values: &[f64]) -> Vec<NonNegativeAmount> {
    values
        .iter()
        .map(|value| match NonNegativeAmount::try_from(*value) {
            Ok(amount) => amount,
            Err(error) => panic!("fixture must parse: {error}"),
        })
        .collect()
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
