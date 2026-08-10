use incidence_core::identity::SubstanceId;
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::{AccumulationError, NumericalSemanticsVersion};
use incidence_core::presence::ValueState;
use incidence_core::sparse_substance_vector::{SparseSubstanceVector, SparseSubstanceVectorError};
use incidence_core::substance_registry::SubstanceRegistry;

fn id(value: &str) -> SubstanceId {
    match SubstanceId::parse(value) {
        Ok(identity) => identity,
        Err(error) => panic!("fixture must parse: {error}"),
    }
}

fn parse_ids(values: &[&str]) -> Vec<SubstanceId> {
    values.iter().map(|value| id(value)).collect()
}

fn amount(value: f64) -> NonNegativeAmount {
    match NonNegativeAmount::try_from(value) {
        Ok(amount) => amount,
        Err(error) => panic!("fixture must parse: {error}"),
    }
}

fn registry(values: &[&str]) -> SubstanceRegistry {
    match SubstanceRegistry::new(parse_ids(values)) {
        Ok(registry) => registry,
        Err(error) => panic!("fixture must parse: {error}"),
    }
}

fn vector(
    registry: &SubstanceRegistry,
    entries: impl IntoIterator<Item = (SubstanceId, NonNegativeAmount)>,
) -> SparseSubstanceVector {
    match SparseSubstanceVector::new(registry, entries) {
        Ok(vector) => vector,
        Err(error) => panic!("fixture vector must construct: {error}"),
    }
}

#[test]
fn rejects_duplicate_and_unregistered_keys_before_sparse_omission() {
    let registry = registry(&["salt", "water"]);
    let duplicate = SparseSubstanceVector::new(
        &registry,
        [
            (id("water"), NonNegativeAmount::ZERO),
            (id("water"), amount(1.0)),
        ],
    );
    let expected_duplicate = Err(SparseSubstanceVectorError::DuplicateSubstance {
        substance: id("water"),
    });
    assert_eq!(duplicate, expected_duplicate);
    match duplicate {
        Err(error) => assert_eq!(
            error.to_string(),
            "duplicate substance identity `water` in sparse vector"
        ),
        Ok(_) => panic!("duplicate fixture must be rejected"),
    }

    let unregistered =
        SparseSubstanceVector::new(&registry, [(id("nitrate"), NonNegativeAmount::ZERO)]);
    let expected_unregistered = Err(SparseSubstanceVectorError::UnregisteredSubstance {
        substance: id("nitrate"),
    });
    assert_eq!(unregistered, expected_unregistered);
    match unregistered {
        Err(error) => assert_eq!(
            error.to_string(),
            "substance identity `nitrate` is not registered"
        ),
        Ok(_) => panic!("unregistered fixture must be rejected"),
    }
}

#[test]
fn canonical_sparse_iteration_and_accumulation_follow_registry_order() {
    let registry = registry(&["water", "phosphorus", "salt", "nitrate"]);
    let first = vector(
        &registry,
        [
            (id("water"), amount(10_000_000_000_000_000.0)),
            (id("phosphorus"), amount(-0.0)),
            (id("salt"), amount(1.0)),
            (id("nitrate"), amount(1.0)),
        ],
    );
    let expected = [
        ("nitrate", 0x3ff0000000000000),
        ("salt", 0x3ff0000000000000),
        ("water", 0x4341c37937e08000),
    ];
    let first_entries: Vec<_> = first
        .iter()
        .map(|(substance, amount)| (substance.as_str(), amount.value().to_bits()))
        .collect();
    assert_eq!(first_entries, expected);

    let second = vector(
        &registry,
        [
            (id("nitrate"), amount(1.0)),
            (id("salt"), amount(1.0)),
            (id("phosphorus"), amount(0.0)),
            (id("water"), amount(10_000_000_000_000_000.0)),
        ],
    );
    let second_entries: Vec<_> = second
        .iter()
        .map(|(substance, amount)| (substance.as_str(), amount.value().to_bits()))
        .collect();
    assert_eq!(second, first);
    assert_eq!(second_entries, expected);

    let first_sum = match first.accumulate(NumericalSemanticsVersion::V1) {
        Ok(sum) => sum,
        Err(error) => panic!("fixture must accumulate: {error}"),
    };
    let second_sum = match second.accumulate(NumericalSemanticsVersion::V1) {
        Ok(sum) => sum,
        Err(error) => panic!("fixture must accumulate: {error}"),
    };
    assert_eq!(first_sum.value().to_bits(), 0x4341c37937e08001);
    assert_eq!(second_sum.value().to_bits(), 0x4341c37937e08001);
}

#[test]
fn ordered_accumulation_propagates_the_exact_v1_overflow() {
    let registry = registry(&["b", "a"]);
    let vector = vector(
        &registry,
        [(id("b"), amount(f64::MAX)), (id("a"), amount(f64::MAX))],
    );

    let AccumulationError::NonFiniteSum {
        version,
        index,
        partial_sum,
        addend,
    } = (match vector.accumulate(NumericalSemanticsVersion::V1) {
        Ok(_) => panic!("expected non-finite sum error"),
        Err(error) => error,
    })
    else {
        panic!("expected non-finite sum error");
    };

    assert_eq!(version, NumericalSemanticsVersion::V1);
    assert_eq!(index, 1);
    assert_eq!(partial_sum.to_bits(), 0x7fefffffffffffff);
    assert_eq!(addend.to_bits(), 0x7fefffffffffffff);
}

#[test]
fn lookup_distinguishes_modelled_zero_from_not_modelled() {
    let registry = registry(&["water", "salt"]);
    let vector = vector(
        &registry,
        [(id("salt"), amount(4.25)), (id("water"), amount(-0.0))],
    );

    let water = vector.amount(&id("water"));
    assert_eq!(water, ValueState::Present(NonNegativeAmount::ZERO));
    let ValueState::Present(water_amount) = water else {
        panic!("registered omitted substance must be present");
    };
    assert_eq!(water_amount.value().to_bits(), 0x0000000000000000);

    let salt = vector.amount(&id("salt"));
    assert_eq!(salt, ValueState::Present(amount(4.25)));
    let ValueState::Present(salt_amount) = salt else {
        panic!("registered stored substance must be present");
    };
    assert_eq!(salt_amount.value().to_bits(), 0x4011000000000000);

    assert_eq!(vector.amount(&id("nitrate")), ValueState::NotModelled);
    assert_eq!(vector.registry(), &registry);
}
