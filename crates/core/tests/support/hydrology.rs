#![allow(clippy::expect_used)]
//! hydrology_document : () → ModelDocument

use incidence_core::forcing::ForcingSeries;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::interpolation_table::{InterpolationBoundaryPolicy, InterpolationTable};
use incidence_core::model_document::{
    CalendarDocument, ConnectionDocument, HorizonDocument, InitialStockDocument,
    InputBindingDocument, InputSourceDocument, ModelDocument, ModelDocumentVersion,
    ModelVersionsDocument, ParameterDocument, RuleDocument, SubstanceAmountDocument,
    TransferBindingDocument, UnitDocument,
};
use incidence_core::numerical_semantics::{NumericalSemanticsVersion, ScalarComparison};
use incidence_core::partition_expression::PartitionExpr;
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
use incidence_core::temporal::{RunHorizon, TimestepIndex};
use incidence_core::versions::RuleIrVersion;

const R: RuleIrVersion = RuleIrVersion::V1;
const S: NumericalSemanticsVersion = NumericalSemanticsVersion::V1;

pub const RULE_COMPARTMENTS: [&str; 7] = [
    "muskingum",
    "lag",
    "linear-reservoir",
    "evaporation",
    "seepage",
    "reservoir-policy",
    "demand",
];

fn compartment(value: &str) -> CompartmentId {
    CompartmentId::parse(value)
        .unwrap_or_else(|error| panic!("invalid compartment {value}: {error}"))
}
fn substance(value: &str) -> SubstanceId {
    SubstanceId::parse(value).unwrap_or_else(|error| panic!("invalid substance {value}: {error}"))
}
fn forcing_id(value: &str) -> ForcingId {
    ForcingId::parse(value).unwrap_or_else(|error| panic!("invalid forcing {value}: {error}"))
}
fn input_id(value: &str) -> InputId {
    InputId::parse(value).unwrap_or_else(|error| panic!("invalid input {value}: {error}"))
}
fn parameter_id(value: &str) -> ParameterId {
    ParameterId::parse(value).unwrap_or_else(|error| panic!("invalid parameter {value}: {error}"))
}
fn projection_id(value: &str) -> ProjectionId {
    ProjectionId::parse(value).unwrap_or_else(|error| panic!("invalid projection {value}: {error}"))
}
fn table_id(value: &str) -> TableId {
    TableId::parse(value).unwrap_or_else(|error| panic!("invalid table {value}: {error}"))
}
fn branch_id(value: &str) -> TransferBranchId {
    TransferBranchId::parse(value).unwrap_or_else(|error| panic!("invalid branch {value}: {error}"))
}
fn literal(value: f64) -> RuleExpr {
    RuleExpr::literal(R, S, value).unwrap_or_else(|error| panic!("invalid literal: {error}"))
}
fn input(value: &str) -> RuleExpr {
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
    RuleExpr::add(lhs, rhs).unwrap_or_else(|error| panic!("invalid addition: {error}"))
}
fn subtract(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::subtract(lhs, rhs).unwrap_or_else(|error| panic!("invalid subtraction: {error}"))
}
fn multiply(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::multiply(lhs, rhs).unwrap_or_else(|error| panic!("invalid multiplication: {error}"))
}
fn divide(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::divide(lhs, rhs).unwrap_or_else(|error| panic!("invalid division: {error}"))
}
fn minimum(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::minimum(lhs, rhs).unwrap_or_else(|error| panic!("invalid minimum: {error}"))
}
fn maximum(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    RuleExpr::maximum(lhs, rhs).unwrap_or_else(|error| panic!("invalid maximum: {error}"))
}
fn state(id: &str, values: Vec<InitialProjectionValue>) -> InitialProjectorState {
    InitialProjectorState::new(projection_id(id), S, values)
        .unwrap_or_else(|error| panic!("invalid state {id}: {error}"))
}
fn incoming(owner: &str) -> AuthoritativeFactSelector {
    AuthoritativeFactSelector::IncomingTransferAmount {
        compartment: compartment(owner),
        substance: substance("water"),
    }
}
fn outgoing(owner: &str) -> AuthoritativeFactSelector {
    AuthoritativeFactSelector::OutgoingTransferAmount {
        compartment: compartment(owner),
        substance: substance("water"),
    }
}

fn projection_document() -> ProjectionSet {
    let muskingum_in: ProjectionSpec = BoundedLagSpec::new(
        R,
        S,
        projection_id("muskingum-in-lag"),
        ProjectionSource::AuthoritativeFact(incoming("muskingum")),
        1,
    )
    .expect("Muskingum incoming lag")
    .into();
    let muskingum_out: ProjectionSpec = BoundedLagSpec::new(
        R,
        S,
        projection_id("muskingum-out-lag"),
        ProjectionSource::AuthoritativeFact(outgoing("muskingum")),
        1,
    )
    .expect("Muskingum outgoing lag")
    .into();
    let update = add(
        add(
            multiply(parameter("muskingum-c0"), input("muskingum-current-in")),
            multiply(parameter("muskingum-c1"), projection("muskingum-in-lag")),
        ),
        multiply(parameter("muskingum-c2"), projection("muskingum-out-lag")),
    );
    let muskingum: ProjectionSpec = FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("muskingum-routed"),
        vec![ProjectionValueKind::Extensive],
        vec![RecurrenceInputBinding::new(
            InputRef::new(
                input_id("muskingum-current-in"),
                ExpressionValueKind::Scalar,
            ),
            RecurrenceInputSource::AuthoritativeFact(incoming("muskingum")),
        )],
        ["muskingum-c0", "muskingum-c1", "muskingum-c2"]
            .into_iter()
            .map(|id| ParameterRef::new(parameter_id(id), ExpressionValueKind::Scalar))
            .collect(),
        vec![update],
        0,
    )
    .expect("Muskingum recurrence")
    .into();
    let lag: ProjectionSpec = BoundedLagSpec::new(
        R,
        S,
        projection_id("travel-time-lag"),
        ProjectionSource::AuthoritativeFact(incoming("lag")),
        2,
    )
    .expect("travel-time lag")
    .into();
    let linear_previous = input("linear-previous-storage");
    let linear_coefficient = parameter("linear-coefficient");
    let linear_storage_update = multiply(
        subtract(literal(1.0), linear_coefficient.clone()),
        linear_previous.clone(),
    );
    let linear_release_update = multiply(linear_coefficient, linear_previous);
    let linear: ProjectionSpec = FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("linear-reservoir-state"),
        vec![
            ProjectionValueKind::Extensive,
            ProjectionValueKind::Extensive,
        ],
        vec![RecurrenceInputBinding::new(
            InputRef::new(
                input_id("linear-previous-storage"),
                ExpressionValueKind::Scalar,
            ),
            RecurrenceInputSource::PreviousState {
                index: 0,
                value_kind: ProjectionValueKind::Extensive,
            },
        )],
        vec![ParameterRef::new(
            parameter_id("linear-coefficient"),
            ExpressionValueKind::Scalar,
        )],
        vec![linear_storage_update, linear_release_update],
        1,
    )
    .expect("linear reservoir recurrence")
    .into();
    ProjectionSet::new(
        vec![muskingum_in, muskingum_out, muskingum, lag, linear],
        vec![
            state(
                "muskingum-in-lag",
                vec![InitialProjectionValue::Extensive(4.0)],
            ),
            state(
                "muskingum-out-lag",
                vec![InitialProjectionValue::Extensive(2.0)],
            ),
            state(
                "muskingum-routed",
                vec![InitialProjectionValue::Extensive(0.0)],
            ),
            state(
                "travel-time-lag",
                vec![
                    InitialProjectionValue::Extensive(3.0),
                    InitialProjectionValue::Extensive(2.0),
                ],
            ),
            state(
                "linear-reservoir-state",
                vec![
                    InitialProjectionValue::Extensive(8.0),
                    InitialProjectionValue::Extensive(0.0),
                ],
            ),
        ],
    )
    .expect("hydrology projection set")
}

fn rule(compartment: &str, expression: RuleExpr, parameters: &[(&str, f64)]) -> RuleDocument {
    RuleDocument {
        compartment: compartment.to_owned(),
        substance: "water".to_owned(),
        expression,
        disposition: PartitionExpr::release_all(R, S, branch_id(&format!("{compartment}-out"))),
        parameters: parameters
            .iter()
            .map(|(id, value)| ParameterDocument {
                id: (*id).to_owned(),
                value: *value,
            })
            .collect(),
    }
}

#[must_use]
pub fn fixture() -> ModelDocument {
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(2)).expect("horizon");
    let forcings = [
        ("air-temperature", vec![0.0, 10.0, 20.0]),
        ("seepage-available", vec![10.0, 10.0, 10.0]),
        ("hydraulic-head", vec![4.0, 6.0, 8.0]),
        ("policy-storage", vec![40.0, 60.0, 45.0]),
        ("policy-available", vec![10.0, 10.0, 10.0]),
        ("requested-demand", vec![6.0, 8.0, 10.0]),
        ("demand-available", vec![20.0, 20.0, 20.0]),
    ]
    .into_iter()
    .map(|(id, values)| ForcingSeries::new(forcing_id(id), horizon, values).expect("forcing"))
    .collect();
    let table = InterpolationTable::new(
        table_id("evaporation-by-temperature"),
        S,
        InterpolationBoundaryPolicy::ClampToEndpoint,
        vec![0.0, 10.0, 20.0],
        vec![1.0, 2.0, 3.0],
    )
    .expect("evaporation table");
    let evaporation = RuleExpr::interpolated_table(
        InterpolatedTableRef::new(table_id("evaporation-by-temperature")),
        forcing("air-temperature"),
    )
    .expect("evaporation lookup");
    let seepage = minimum(
        input("seepage-available"),
        maximum(
            literal(0.0),
            multiply(parameter("seepage-conductance"), input("hydraulic-head")),
        ),
    );
    let policy_condition = RuleExpr::comparison(
        ScalarComparison::GreaterThanOrEqual,
        input("policy-storage"),
        parameter("flood-threshold"),
    )
    .expect("policy comparison");
    let selected = RuleExpr::select(
        policy_condition,
        parameter("flood-release"),
        parameter("conservation-release"),
    )
    .expect("policy select");
    let policy =
        RuleExpr::clamp(selected, literal(0.0), input("policy-available")).expect("policy clamp");
    let demand = minimum(
        forcing("requested-demand"),
        divide(input("demand-available"), parameter("delivery-period")),
    );
    let rules = vec![
        rule(
            "muskingum",
            projection("muskingum-routed"),
            &[
                ("muskingum-c0", 0.4),
                ("muskingum-c1", 0.3),
                ("muskingum-c2", 0.3),
            ],
        ),
        rule("lag", projection("travel-time-lag"), &[]),
        rule(
            "linear-reservoir",
            projection("linear-reservoir-state"),
            &[("linear-coefficient", 0.25)],
        ),
        rule("evaporation", evaporation, &[]),
        rule("seepage", seepage, &[("seepage-conductance", 0.5)]),
        rule(
            "reservoir-policy",
            policy,
            &[
                ("flood-threshold", 50.0),
                ("flood-release", 8.0),
                ("conservation-release", 3.0),
            ],
        ),
        rule("demand", demand, &[("delivery-period", 2.0)]),
    ];
    let transfer_bindings = RULE_COMPARTMENTS
        .into_iter()
        .map(|owner| TransferBindingDocument {
            compartment: owner.to_owned(),
            substance: "water".to_owned(),
            branch: format!("{owner}-out"),
            destination: "outside".to_owned(),
        })
        .collect();
    let input_bindings = [
        ("seepage", "seepage-available", "seepage-available"),
        ("seepage", "hydraulic-head", "hydraulic-head"),
        ("reservoir-policy", "policy-storage", "policy-storage"),
        ("reservoir-policy", "policy-available", "policy-available"),
        ("demand", "demand-available", "demand-available"),
    ]
    .into_iter()
    .map(|(owner, input, forcing)| InputBindingDocument {
        compartment: owner.to_owned(),
        substance: "water".to_owned(),
        input: input.to_owned(),
        value_kind: ExpressionValueKind::Scalar,
        source: InputSourceDocument::Forcing {
            forcing: forcing.to_owned(),
        },
    })
    .collect();
    ModelDocument {
        document_version: ModelDocumentVersion::V1,
        versions: ModelVersionsDocument::default(),
        finite_compartments: RULE_COMPARTMENTS.into_iter().map(str::to_owned).collect(),
        boundary_accounts: vec!["outside".to_owned()],
        connections: RULE_COMPARTMENTS
            .into_iter()
            .map(|id| ConnectionDocument {
                source: id.to_owned(),
                target: "outside".to_owned(),
            })
            .collect(),
        substances: vec!["water".to_owned()],
        initial_stocks: RULE_COMPARTMENTS
            .into_iter()
            .map(|id| InitialStockDocument {
                compartment: id.to_owned(),
                amounts: vec![SubstanceAmountDocument {
                    substance: "water".to_owned(),
                    amount: 100.0,
                }],
            })
            .collect(),
        calendar: CalendarDocument {
            origin_unix_seconds: 0,
            timestep_seconds: 86_400,
        },
        horizon: HorizonDocument { first: 0, last: 2 },
        projections: projection_document(),
        forcings,
        interpolation_tables: vec![table],
        rules,
        transfer_bindings,
        input_bindings,
        units: vec![UnitDocument {
            substance: "water".to_owned(),
            unit: "m3".to_owned(),
        }],
    }
}
