//! Proof fixtures showing that the neutral core IR can express representative conserved-flow rules.

use std::fmt::Debug;

use incidence_core::canonical_encoding::CanonicalEncode;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::numerical_semantics::{NumericalSemanticsVersion, ScalarComparison};
use incidence_core::partition_expression::{Fraction, FractionBranch, PartitionExpr};
use incidence_core::projection::{
    AuthoritativeFactSelector, BoundedLagSpec, FiniteRecurrenceSpec, InitialProjectionValue,
    InitialProjectorState, ProjectionSet, ProjectionSource, ProjectionSpec, RecurrenceInputBinding,
    RecurrenceInputSource,
};
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef,
    ParameterId, ParameterRef, ProjectionId, ProjectionRef, ProjectionValueKind, TableId,
    TransferBranchId,
};
use incidence_core::versions::{CanonicalEncodingVersion, RuleIrVersion};
use serde::{Serialize, de::DeserializeOwned};

const R: RuleIrVersion = RuleIrVersion::V1;
const S: NumericalSemanticsVersion = NumericalSemanticsVersion::V1;
const C: CanonicalEncodingVersion = CanonicalEncodingVersion::V1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixtureName {
    Muskingum,
    Lag,
    LinearReservoir,
    TableInterpolatedEvaporation,
    Seepage,
    ReservoirOperatingPolicy,
    Demand,
}

struct HydrologyFixture {
    name: FixtureName,
    expression: RuleExpr,
    partition: PartitionExpr,
    projections: Option<ProjectionSet>,
}

fn input_id(value: &str) -> InputId {
    InputId::parse(value).unwrap_or_else(|error| panic!("invalid fixture input {value}: {error}"))
}
fn parameter_id(value: &str) -> ParameterId {
    ParameterId::parse(value)
        .unwrap_or_else(|error| panic!("invalid fixture parameter {value}: {error}"))
}
fn forcing_id(value: &str) -> ForcingId {
    ForcingId::parse(value)
        .unwrap_or_else(|error| panic!("invalid fixture forcing {value}: {error}"))
}
fn projection_id(value: &str) -> ProjectionId {
    ProjectionId::parse(value)
        .unwrap_or_else(|error| panic!("invalid fixture projection {value}: {error}"))
}
fn table_id(value: &str) -> TableId {
    TableId::parse(value).unwrap_or_else(|error| panic!("invalid fixture table {value}: {error}"))
}
fn branch_id(value: &str) -> TransferBranchId {
    TransferBranchId::parse(value)
        .unwrap_or_else(|error| panic!("invalid fixture branch {value}: {error}"))
}
fn compartment(value: &str) -> CompartmentId {
    CompartmentId::parse(value)
        .unwrap_or_else(|error| panic!("invalid fixture compartment {value}: {error}"))
}
fn substance(value: &str) -> SubstanceId {
    SubstanceId::parse(value)
        .unwrap_or_else(|error| panic!("invalid fixture substance {value}: {error}"))
}
fn scalar_input(value: &str) -> RuleExpr {
    RuleExpr::input(
        R,
        S,
        InputRef::new(input_id(value), ExpressionValueKind::Scalar),
    )
}
fn parameter(value: &str) -> RuleExpr {
    RuleExpr::parameter(
        R,
        S,
        ParameterRef::new(parameter_id(value), ExpressionValueKind::Scalar),
    )
}
fn forcing(value: &str) -> RuleExpr {
    RuleExpr::forcing(R, S, ForcingRef::new(forcing_id(value)))
}
fn projection(value: &str) -> RuleExpr {
    RuleExpr::projection(
        R,
        S,
        ProjectionRef::new(projection_id(value), ProjectionValueKind::Extensive),
    )
}
fn add(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::add(lhs, rhs).unwrap_or_else(|error| panic!("invalid fixture addition: {error}"))
}
fn subtract(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::subtract(lhs, rhs)
        .unwrap_or_else(|error| panic!("invalid fixture subtraction: {error}"))
}
fn multiply(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::multiply(lhs, rhs)
        .unwrap_or_else(|error| panic!("invalid fixture multiplication: {error}"))
}
fn divide(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::divide(lhs, rhs).unwrap_or_else(|error| panic!("invalid fixture division: {error}"))
}
fn minimum(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::minimum(lhs, rhs).unwrap_or_else(|error| panic!("invalid fixture minimum: {error}"))
}
fn maximum(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::maximum(lhs, rhs).unwrap_or_else(|error| panic!("invalid fixture maximum: {error}"))
}
fn clamp(value: RuleExpr, lower: RuleExpr, upper: RuleExpr) -> RuleExpr {
    RuleExpr::clamp(value, lower, upper)
        .unwrap_or_else(|error| panic!("invalid fixture clamp: {error}"))
}
fn fraction(value: f64) -> Fraction {
    Fraction::new(S, value)
        .unwrap_or_else(|error| panic!("invalid fixture fraction {value}: {error}"))
}
fn state(id: &str, values: Vec<InitialProjectionValue>) -> InitialProjectorState {
    InitialProjectorState::new(projection_id(id), S, values)
        .unwrap_or_else(|error| panic!("invalid fixture state {id}: {error}"))
}
fn incoming(compartment_id: &str) -> AuthoritativeFactSelector {
    AuthoritativeFactSelector::IncomingTransferAmount {
        compartment: compartment(compartment_id),
        substance: substance("water"),
    }
}
fn outgoing(compartment_id: &str) -> AuthoritativeFactSelector {
    AuthoritativeFactSelector::OutgoingTransferAmount {
        compartment: compartment(compartment_id),
        substance: substance("water"),
    }
}
fn release(branch: &str) -> PartitionExpr {
    PartitionExpr::release_all(R, S, branch_id(branch))
}

fn muskingum_fixture() -> HydrologyFixture {
    let incoming_lag: ProjectionSpec = BoundedLagSpec::new(
        R,
        S,
        projection_id("muskingum-incoming-lag"),
        ProjectionSource::AuthoritativeFact(incoming("reach")),
        1,
    )
    .unwrap_or_else(|error| panic!("invalid Muskingum incoming lag: {error}"))
    .into();
    let outgoing_lag: ProjectionSpec = BoundedLagSpec::new(
        R,
        S,
        projection_id("muskingum-outgoing-lag"),
        ProjectionSource::AuthoritativeFact(outgoing("reach")),
        1,
    )
    .unwrap_or_else(|error| panic!("invalid Muskingum outgoing lag: {error}"))
    .into();
    let update = add(
        add(
            multiply(parameter("muskingum-c0"), scalar_input("incoming-current")),
            multiply(
                parameter("muskingum-c1"),
                projection("muskingum-incoming-lag"),
            ),
        ),
        multiply(
            parameter("muskingum-c2"),
            projection("muskingum-outgoing-lag"),
        ),
    );
    let routed: ProjectionSpec = FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("muskingum-routed"),
        vec![ProjectionValueKind::Extensive],
        vec![RecurrenceInputBinding::new(
            InputRef::new(input_id("incoming-current"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::AuthoritativeFact(incoming("reach")),
        )],
        ["muskingum-c0", "muskingum-c1", "muskingum-c2"]
            .into_iter()
            .map(|id| ParameterRef::new(parameter_id(id), ExpressionValueKind::Scalar))
            .collect(),
        vec![update.clone()],
        0,
    )
    .unwrap_or_else(|error| panic!("invalid Muskingum recurrence: {error}"))
    .into();
    let projections = ProjectionSet::new(
        vec![incoming_lag, outgoing_lag, routed],
        vec![
            state(
                "muskingum-incoming-lag",
                vec![InitialProjectionValue::Extensive(0.0)],
            ),
            state(
                "muskingum-outgoing-lag",
                vec![InitialProjectionValue::Extensive(0.0)],
            ),
            state(
                "muskingum-routed",
                vec![InitialProjectionValue::Extensive(0.0)],
            ),
        ],
    )
    .unwrap_or_else(|error| panic!("invalid Muskingum projection set: {error}"));
    HydrologyFixture {
        name: FixtureName::Muskingum,
        expression: update,
        partition: release("routed-flow"),
        projections: Some(projections),
    }
}

fn lag_fixture() -> HydrologyFixture {
    let spec: ProjectionSpec = BoundedLagSpec::new(
        R,
        S,
        projection_id("travel-time-lag"),
        ProjectionSource::AuthoritativeFact(incoming("conduit")),
        3,
    )
    .unwrap_or_else(|error| panic!("invalid lag projection: {error}"))
    .into();
    let projections = ProjectionSet::new(
        vec![spec],
        vec![state(
            "travel-time-lag",
            vec![
                InitialProjectionValue::Extensive(0.0),
                InitialProjectionValue::Extensive(0.0),
                InitialProjectionValue::Extensive(0.0),
            ],
        )],
    )
    .unwrap_or_else(|error| panic!("invalid lag projection set: {error}"));
    HydrologyFixture {
        name: FixtureName::Lag,
        expression: projection("travel-time-lag"),
        partition: PartitionExpr::retain_all(R, S),
        projections: Some(projections),
    }
}

fn linear_reservoir_fixture() -> HydrologyFixture {
    let previous = scalar_input("previous-storage");
    let release_amount = multiply(parameter("release-coefficient"), previous.clone());
    let update = add(
        scalar_input("reservoir-inflow"),
        subtract(previous, release_amount),
    );
    let spec: ProjectionSpec = FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("linear-reservoir-state"),
        vec![ProjectionValueKind::Extensive],
        vec![
            RecurrenceInputBinding::new(
                InputRef::new(input_id("previous-storage"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::PreviousState {
                    index: 0,
                    value_kind: ProjectionValueKind::Extensive,
                },
            ),
            RecurrenceInputBinding::new(
                InputRef::new(input_id("reservoir-inflow"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(incoming("reservoir")),
            ),
        ],
        vec![ParameterRef::new(
            parameter_id("release-coefficient"),
            ExpressionValueKind::Scalar,
        )],
        vec![update.clone()],
        0,
    )
    .unwrap_or_else(|error| panic!("invalid linear reservoir recurrence: {error}"))
    .into();
    let projections = ProjectionSet::new(
        vec![spec],
        vec![state(
            "linear-reservoir-state",
            vec![InitialProjectionValue::Extensive(10.0)],
        )],
    )
    .unwrap_or_else(|error| panic!("invalid linear reservoir projection set: {error}"));
    HydrologyFixture {
        name: FixtureName::LinearReservoir,
        expression: update,
        partition: PartitionExpr::constant_fraction_transfer(
            R,
            S,
            branch_id("linear-release"),
            fraction(0.25),
        )
        .unwrap_or_else(|error| panic!("invalid linear reservoir partition: {error}")),
        projections: Some(projections),
    }
}

fn table_interpolated_evaporation_fixture() -> HydrologyFixture {
    let expression = RuleExpr::interpolated_table(
        InterpolatedTableRef::new(table_id("evaporation-by-temperature")),
        forcing("air-temperature"),
    )
    .unwrap_or_else(|error| panic!("invalid evaporation lookup: {error}"));
    HydrologyFixture {
        name: FixtureName::TableInterpolatedEvaporation,
        expression,
        partition: PartitionExpr::exogenous_series(
            R,
            S,
            branch_id("evaporation"),
            ForcingRef::new(forcing_id("potential-evaporation")),
        ),
        projections: None,
    }
}

fn seepage_fixture() -> HydrologyFixture {
    let expression = minimum(
        scalar_input("available-storage"),
        maximum(
            RuleExpr::literal(R, S, 0.0)
                .unwrap_or_else(|error| panic!("invalid zero literal: {error}")),
            multiply(
                parameter("seepage-conductance"),
                scalar_input("hydraulic-head"),
            ),
        ),
    );
    let partition = PartitionExpr::fixed_fraction_split(
        R,
        S,
        fraction(0.75),
        vec![FractionBranch::new(branch_id("seepage"), fraction(0.25))],
    )
    .unwrap_or_else(|error| panic!("invalid seepage partition: {error}"));
    HydrologyFixture {
        name: FixtureName::Seepage,
        expression,
        partition,
        projections: None,
    }
}

fn reservoir_operating_policy_fixture() -> HydrologyFixture {
    let condition = RuleExpr::comparison(
        ScalarComparison::GreaterThanOrEqual,
        scalar_input("reservoir-storage"),
        parameter("flood-pool-threshold"),
    )
    .unwrap_or_else(|error| panic!("invalid operating-policy condition: {error}"));
    let selected = RuleExpr::select(
        condition,
        parameter("flood-release"),
        parameter("conservation-release"),
    )
    .unwrap_or_else(|error| panic!("invalid operating-policy select: {error}"));
    let expression = clamp(
        selected,
        RuleExpr::literal(R, S, 0.0)
            .unwrap_or_else(|error| panic!("invalid policy zero literal: {error}")),
        scalar_input("available-release"),
    );
    HydrologyFixture {
        name: FixtureName::ReservoirOperatingPolicy,
        expression,
        partition: release("policy-release"),
        projections: None,
    }
}

fn demand_fixture() -> HydrologyFixture {
    HydrologyFixture {
        name: FixtureName::Demand,
        expression: minimum(
            forcing("requested-demand"),
            divide(
                scalar_input("available-supply"),
                parameter("delivery-period"),
            ),
        ),
        partition: PartitionExpr::exogenous_series(
            R,
            S,
            branch_id("demand-withdrawal"),
            ForcingRef::new(forcing_id("requested-demand")),
        ),
        projections: None,
    }
}

fn fixtures() -> Vec<HydrologyFixture> {
    vec![
        muskingum_fixture(),
        lag_fixture(),
        linear_reservoir_fixture(),
        table_interpolated_evaporation_fixture(),
        seepage_fixture(),
        reservoir_operating_policy_fixture(),
        demand_fixture(),
    ]
}

fn assert_stable_round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + Eq + Debug + CanonicalEncode,
{
    let json = serde_json::to_vec(value)
        .unwrap_or_else(|error| panic!("fixture serialization failed: {error}"));
    let decoded: T = serde_json::from_slice(&json)
        .unwrap_or_else(|error| panic!("fixture deserialization failed: {error}"));
    assert_eq!(&decoded, value);
    let reserialized = serde_json::to_vec(&decoded)
        .unwrap_or_else(|error| panic!("fixture reserialization failed: {error}"));
    assert_eq!(reserialized, json);
    let authored_bytes = C
        .encode(value)
        .unwrap_or_else(|error| panic!("fixture canonical encoding failed: {error}"));
    let decoded_bytes = C
        .encode(&decoded)
        .unwrap_or_else(|error| panic!("round-tripped canonical encoding failed: {error}"));
    assert_eq!(decoded_bytes, authored_bytes);
}

#[test]
fn seven_named_hydrology_fixtures_are_expressible_in_neutral_ir() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 7);
    assert_eq!(
        fixtures
            .iter()
            .map(|fixture| fixture.name)
            .collect::<Vec<_>>(),
        vec![
            FixtureName::Muskingum,
            FixtureName::Lag,
            FixtureName::LinearReservoir,
            FixtureName::TableInterpolatedEvaporation,
            FixtureName::Seepage,
            FixtureName::ReservoirOperatingPolicy,
            FixtureName::Demand,
        ]
    );
    for fixture in fixtures {
        assert_stable_round_trip(&fixture.expression);
        assert_stable_round_trip(&fixture.partition);
        if let Some(projections) = fixture.projections {
            let json = serde_json::to_vec(&projections)
                .unwrap_or_else(|error| panic!("projection set serialization failed: {error}"));
            let decoded: ProjectionSet = serde_json::from_slice(&json)
                .unwrap_or_else(|error| panic!("projection set deserialization failed: {error}"));
            assert_eq!(decoded, projections);
            assert_eq!(
                serde_json::to_vec(&decoded).unwrap_or_else(|error| panic!(
                    "projection set reserialization failed: {error}"
                )),
                json,
            );
            for (authored, round_tripped) in projections.iter().zip(decoded.iter()) {
                assert_stable_round_trip(authored);
                assert_stable_round_trip(round_tripped);
            }
            for ((_, authored), (_, round_tripped)) in
                projections.initial_states().zip(decoded.initial_states())
            {
                assert_stable_round_trip(authored);
                assert_stable_round_trip(round_tripped);
            }
        }
    }
}
