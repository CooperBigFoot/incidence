//! Black-box construction, invariant, serde, and public-shape tests for projections.

use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::numerical_semantics::{NumericalSemanticsVersion, ScalarComparison};
use incidence_core::projection::{
    AuthoritativeFactSelector, BoundedLagSpec, FiniteRecurrenceSpec, ForbiddenRecurrenceLeaf,
    InitialProjectionValue, InitialProjectorState, OrderedRollingAggregateSpec, ProjectionBound,
    ProjectionError, ProjectionSet, ProjectionSource, ProjectionSpec, ProjectionSpecView,
    ProjectionValueKind, RecurrenceInputBinding, RecurrenceInputSource, RollingAggregate,
};
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef,
    ParameterId, ParameterRef, ProjectionId, ProjectionRef, TableId,
};
use incidence_core::versions::RuleIrVersion;

const R: RuleIrVersion = RuleIrVersion::V1;
const S: NumericalSemanticsVersion = NumericalSemanticsVersion::V1;

fn projection_id(value: &str) -> ProjectionId {
    match ProjectionId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("invalid projection fixture: {error}"),
    }
}
fn input_id(value: &str) -> InputId {
    match InputId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("invalid input fixture: {error}"),
    }
}
fn parameter_id(value: &str) -> ParameterId {
    match ParameterId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("invalid parameter fixture: {error}"),
    }
}
fn forcing_id(value: &str) -> ForcingId {
    match ForcingId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("invalid forcing fixture: {error}"),
    }
}
fn table_id(value: &str) -> TableId {
    match TableId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("invalid table fixture: {error}"),
    }
}
fn compartment(value: &str) -> CompartmentId {
    match CompartmentId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("invalid compartment fixture: {error}"),
    }
}
fn substance(value: &str) -> SubstanceId {
    match SubstanceId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("invalid substance fixture: {error}"),
    }
}
fn incoming() -> AuthoritativeFactSelector {
    AuthoritativeFactSelector::IncomingTransferAmount {
        compartment: compartment("reach-a"),
        substance: substance("substance-a"),
    }
}
fn state(id: &str, values: Vec<InitialProjectionValue>) -> InitialProjectorState {
    match InitialProjectorState::new(projection_id(id), S, values) {
        Ok(value) => value,
        Err(error) => panic!("invalid state fixture: {error}"),
    }
}
fn lag(
    id: &str,
    source: ProjectionSource,
    steps: usize,
) -> Result<ProjectionSpec, ProjectionError> {
    BoundedLagSpec::new(R, S, projection_id(id), source, steps).map(Into::into)
}
fn parameter(value: &str) -> RuleExpr {
    RuleExpr::parameter(
        R,
        S,
        ParameterRef::new(parameter_id(value), ExpressionValueKind::Scalar),
    )
}
fn projection(value: &str) -> RuleExpr {
    RuleExpr::projection(
        R,
        S,
        ProjectionRef::new(projection_id(value), ProjectionValueKind::Extensive),
    )
}
fn input(value: &str) -> RuleExpr {
    RuleExpr::input(
        R,
        S,
        InputRef::new(input_id(value), ExpressionValueKind::Scalar),
    )
}
fn multiply(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    match RuleExpr::multiply(lhs, rhs) {
        Ok(value) => value,
        Err(error) => panic!("invalid multiply fixture: {error}"),
    }
}
fn add(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    match RuleExpr::add(lhs, rhs) {
        Ok(value) => value,
        Err(error) => panic!("invalid add fixture: {error}"),
    }
}

fn recurrence_dependency(id: &str, reference: &str, kind: ProjectionValueKind) -> ProjectionSpec {
    let update = RuleExpr::projection(R, S, ProjectionRef::new(projection_id(reference), kind));
    match FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id(id),
        vec![ProjectionValueKind::Extensive],
        Vec::new(),
        Vec::new(),
        vec![update],
        0,
    ) {
        Ok(value) => value.into(),
        Err(error) => panic!("recurrence dependency fixture failed: {error}"),
    }
}

fn independent_recurrence(id: &str) -> ProjectionSpec {
    let update = match RuleExpr::literal(R, S, 1.25) {
        Ok(value) => value,
        Err(error) => panic!("independent recurrence literal failed: {error}"),
    };
    match FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id(id),
        vec![ProjectionValueKind::Extensive],
        Vec::new(),
        Vec::new(),
        vec![update],
        0,
    ) {
        Ok(value) => value.into(),
        Err(error) => panic!("independent recurrence fixture failed: {error}"),
    }
}

fn extensive_state(id: &str) -> InitialProjectorState {
    state(id, vec![InitialProjectionValue::Extensive(1.25)])
}

#[test]
fn positive_bounds_and_initial_values_reject_exact_invalid_inputs() {
    assert_eq!(
        lag("lag-a", ProjectionSource::AuthoritativeFact(incoming()), 0),
        Err(ProjectionError::NonPositiveBound {
            projection: projection_id("lag-a"),
            bound: ProjectionBound::BoundedLagSteps,
            attempted: 0
        })
    );
    assert_eq!(
        OrderedRollingAggregateSpec::new(
            R,
            S,
            projection_id("rolling-a"),
            ProjectionSource::AuthoritativeFact(incoming()),
            0,
            RollingAggregate::SumOldestToNewest
        ),
        Err(ProjectionError::NonPositiveBound {
            projection: projection_id("rolling-a"),
            bound: ProjectionBound::RollingWindow,
            attempted: 0
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![],
            vec![],
            vec![],
            vec![],
            0
        ),
        Err(ProjectionError::NonPositiveBound {
            projection: projection_id("rec-a"),
            bound: ProjectionBound::RecurrenceStateSize,
            attempted: 0
        })
    );
    for (value, bits) in [
        (f64::NAN, 0x7ff8_0000_0000_0000),
        (f64::INFINITY, 0x7ff0_0000_0000_0000),
        (f64::NEG_INFINITY, 0xfff0_0000_0000_0000),
    ] {
        assert_eq!(
            InitialProjectorState::new(
                projection_id("lag-a"),
                S,
                vec![InitialProjectionValue::Extensive(value)]
            ),
            Err(ProjectionError::NonFiniteInitialValue {
                projection: projection_id("lag-a"),
                index: 0,
                bits
            })
        );
    }
    let negative = state("lag-a", vec![InitialProjectionValue::Extensive(-0.0)]);
    let positive = state("lag-a", vec![InitialProjectionValue::Extensive(0.0)]);
    assert_eq!(negative, positive);
    assert_eq!(negative.values()[0].bits(), Some(0));
}

#[test]
fn projection_set_rejects_shape_identity_and_dependency_failures() {
    let lag_a = match lag(
        "projection-a",
        ProjectionSource::AuthoritativeFact(incoming()),
        3,
    ) {
        Ok(value) => value,
        Err(error) => panic!("lag fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![lag_a.clone()],
            vec![state(
                "projection-a",
                vec![
                    InitialProjectionValue::Extensive(1.25),
                    InitialProjectionValue::Extensive(2.5)
                ]
            )]
        ),
        Err(ProjectionError::InitialStateLength {
            projection: projection_id("projection-a"),
            expected: 3,
            actual: 2
        })
    );
    assert_eq!(
        ProjectionSet::new(
            vec![lag_a.clone(), lag_a.clone()],
            vec![state("projection-a", vec![])]
        ),
        Err(ProjectionError::DuplicateProjection {
            projection: projection_id("projection-a")
        })
    );
    assert_eq!(
        ProjectionSet::new(vec![lag_a.clone()], vec![]),
        Err(ProjectionError::MissingInitialState {
            projection: projection_id("projection-a")
        })
    );
    let maximal_lag = match lag(
        "maximal-lag",
        ProjectionSource::AuthoritativeFact(incoming()),
        usize::MAX,
    ) {
        Ok(value) => value,
        Err(error) => panic!("maximal lag fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(vec![maximal_lag], vec![state("maximal-lag", vec![])]),
        Err(ProjectionError::InitialStateLength {
            projection: projection_id("maximal-lag"),
            expected: usize::MAX,
            actual: 0,
        })
    );
    assert_eq!(
        ProjectionSet::new(
            vec![lag_a.clone()],
            vec![state("projection-a", vec![]), state("projection-a", vec![])]
        ),
        Err(ProjectionError::DuplicateInitialState {
            projection: projection_id("projection-a")
        })
    );
    assert_eq!(
        ProjectionSet::new(vec![lag_a], vec![state("projection-z", vec![])]),
        Err(ProjectionError::MissingInitialState {
            projection: projection_id("projection-a")
        })
    );

    let first = match lag(
        "projection-a",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-b"),
            ProjectionValueKind::Extensive,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("cycle fixture failed: {error}"),
    };
    let second = match lag(
        "projection-b",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-a"),
            ProjectionValueKind::Extensive,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("cycle fixture failed: {error}"),
    };
    let states = vec![
        state(
            "projection-a",
            vec![InitialProjectionValue::Extensive(1.25)],
        ),
        state("projection-b", vec![InitialProjectionValue::Extensive(2.5)]),
    ];
    let cycle = ProjectionError::CyclicDependency {
        cycle: vec![
            projection_id("projection-a"),
            projection_id("projection-b"),
            projection_id("projection-a"),
        ],
    };
    assert_eq!(
        ProjectionSet::new(vec![first.clone(), second.clone()], states.clone()),
        Err(cycle.clone())
    );
    assert_eq!(ProjectionSet::new(vec![second, first], states), Err(cycle));
}

#[test]
fn muskingum_shape_uses_only_public_deterministic_projection_ir() {
    let incoming_previous = match lag(
        "z-incoming-previous",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("incoming lag failed: {error}"),
    };
    let outgoing_selector = AuthoritativeFactSelector::OutgoingTransferAmount {
        compartment: compartment("reach-a"),
        substance: substance("substance-a"),
    };
    let outgoing_previous = match lag(
        "a-outgoing-previous",
        ProjectionSource::AuthoritativeFact(outgoing_selector),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("outgoing lag failed: {error}"),
    };
    let update = add(
        add(
            multiply(parameter("coefficient-zero"), input("incoming-current")),
            multiply(
                parameter("coefficient-one"),
                projection("z-incoming-previous"),
            ),
        ),
        multiply(
            parameter("coefficient-two"),
            projection("a-outgoing-previous"),
        ),
    );
    const UPDATE_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"add","lhs":{"kind":"add","lhs":{"kind":"multiply","lhs":{"kind":"parameter","reference":{"id":"coefficient-zero","value_kind":"scalar"}},"rhs":{"kind":"input","reference":{"id":"incoming-current","value_kind":"scalar"}}},"rhs":{"kind":"multiply","lhs":{"kind":"parameter","reference":{"id":"coefficient-one","value_kind":"scalar"}},"rhs":{"kind":"projection","reference":{"id":"z-incoming-previous","value_kind":"extensive"}}}},"rhs":{"kind":"multiply","lhs":{"kind":"parameter","reference":{"id":"coefficient-two","value_kind":"scalar"}},"rhs":{"kind":"projection","reference":{"id":"a-outgoing-previous","value_kind":"extensive"}}}}}"#;
    let update_json = match serde_json::to_string(&update) {
        Ok(value) => value,
        Err(error) => panic!("update must serialize: {error}"),
    };
    assert_eq!(update_json, UPDATE_JSON);
    let recurrence = match FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("muskingum-outgoing"),
        vec![ProjectionValueKind::Extensive],
        vec![RecurrenceInputBinding::new(
            InputRef::new(input_id("incoming-current"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::AuthoritativeFact(incoming()),
        )],
        vec![
            ParameterRef::new(
                parameter_id("coefficient-zero"),
                ExpressionValueKind::Scalar,
            ),
            ParameterRef::new(parameter_id("coefficient-two"), ExpressionValueKind::Scalar),
            ParameterRef::new(parameter_id("coefficient-one"), ExpressionValueKind::Scalar),
        ],
        vec![update],
        0,
    ) {
        Ok(value) => value,
        Err(error) => panic!("recurrence fixture failed: {error}"),
    };
    assert_eq!(
        recurrence
            .parameters()
            .iter()
            .map(|p| p.id().as_str())
            .collect::<Vec<_>>(),
        ["coefficient-one", "coefficient-two", "coefficient-zero"]
    );
    assert_eq!(
        recurrence
            .dependencies()
            .map(|p| p.id().as_str())
            .collect::<Vec<_>>(),
        ["z-incoming-previous", "a-outgoing-previous"]
    );
    let mut lexical_dependencies = recurrence
        .dependencies()
        .map(|reference| reference.id().as_str())
        .collect::<Vec<_>>();
    lexical_dependencies.sort_unstable();
    assert_eq!(
        lexical_dependencies,
        ["a-outgoing-previous", "z-incoming-previous"]
    );
    let set = match ProjectionSet::new(
        vec![incoming_previous, outgoing_previous, recurrence.into()],
        vec![
            state(
                "z-incoming-previous",
                vec![InitialProjectionValue::Extensive(11.25)],
            ),
            state(
                "a-outgoing-previous",
                vec![InitialProjectionValue::Extensive(7.75)],
            ),
            state(
                "muskingum-outgoing",
                vec![InitialProjectionValue::Extensive(6.5)],
            ),
        ],
    ) {
        Ok(value) => value,
        Err(error) => panic!("projection set failed: {error}"),
    };
    assert_eq!(
        set.iter()
            .map(|spec| spec.id().as_str())
            .collect::<Vec<_>>(),
        [
            "z-incoming-previous",
            "a-outgoing-previous",
            "muskingum-outgoing"
        ]
    );
    match set.iter().nth(2).map(ProjectionSpec::view) {
        Some(ProjectionSpecView::FiniteRecurrence(spec)) => {
            assert_eq!(spec.value_kind(), ProjectionValueKind::Extensive);
            assert_eq!(spec.state_kinds().len(), 1);
            assert_eq!(spec.updates().len(), 1);
            assert_eq!(spec.output_index(), 0);
        }
        _ => panic!("expected recurrence view"),
    }
    for (id, bits) in [
        ("z-incoming-previous", 11.25_f64.to_bits()),
        ("a-outgoing-previous", 7.75_f64.to_bits()),
        ("muskingum-outgoing", 6.5_f64.to_bits()),
    ] {
        let stored = match set.initial_state(&projection_id(id)) {
            Some(value) => value,
            None => panic!("missing state fixture"),
        };
        assert_eq!(stored.values()[0].bits(), Some(bits));
    }
    let json = match serde_json::to_string(&set) {
        Ok(value) => value,
        Err(error) => panic!("set must serialize: {error}"),
    };
    let decoded: ProjectionSet = match serde_json::from_str(&json) {
        Ok(value) => value,
        Err(error) => panic!("set must decode: {error}"),
    };
    assert_eq!(decoded, set);
    let reserialized = match serde_json::to_string(&decoded) {
        Ok(value) => value,
        Err(error) => panic!("decoded set must serialize: {error}"),
    };
    assert_eq!(reserialized, json);

    let specifications = set.iter().collect::<Vec<_>>();
    for (index, expected_direction) in [(0, "incoming"), (1, "outgoing")] {
        match specifications[index].view() {
            ProjectionSpecView::BoundedLag(spec) => {
                assert_eq!(spec.steps(), 1);
                match spec.source() {
                    ProjectionSource::AuthoritativeFact(selector) => {
                        assert_eq!(selector.compartment().as_str(), "reach-a");
                        assert_eq!(selector.substance().as_str(), "substance-a");
                        match (expected_direction, selector) {
                            (
                                "incoming",
                                AuthoritativeFactSelector::IncomingTransferAmount { .. },
                            )
                            | (
                                "outgoing",
                                AuthoritativeFactSelector::OutgoingTransferAmount { .. },
                            ) => {}
                            _ => panic!("unexpected authoritative selector direction"),
                        }
                    }
                    ProjectionSource::Projection(_) => {
                        panic!("lag must select an authoritative fact")
                    }
                }
            }
            _ => panic!("expected bounded lag view"),
        }
    }
    match specifications[2].view() {
        ProjectionSpecView::FiniteRecurrence(spec) => assert_eq!(
            spec.inputs()
                .iter()
                .map(|binding| binding.reference().id().as_str())
                .collect::<Vec<_>>(),
            ["incoming-current"]
        ),
        _ => panic!("expected recurrence view"),
    }

    let set_wire = match serde_json::to_value(&set) {
        Ok(value) => value,
        Err(error) => panic!("set wire fixture did not serialize: {error}"),
    };
    let incoming_reference: &[&str] = &[
        "specifications",
        "2",
        "spec",
        "updates",
        "0",
        "expression",
        "lhs",
        "rhs",
        "rhs",
        "reference",
    ];
    let outgoing_reference: &[&str] = &[
        "specifications",
        "2",
        "spec",
        "updates",
        "0",
        "expression",
        "rhs",
        "rhs",
        "reference",
    ];
    for path in [incoming_reference, outgoing_reference] {
        let mut malformed = set_wire.clone();
        let mut cursor = &mut malformed;
        for component in path {
            cursor = if let Ok(index) = component.parse::<usize>() {
                &mut cursor[index]
            } else {
                &mut cursor[*component]
            };
        }
        cursor["id"] = serde_json::json!("unknown-projection");
        let expected = ProjectionError::UnknownProjectionReference {
            owner: projection_id("muskingum-outgoing"),
            reference: projection_id("unknown-projection"),
        };
        let error = match serde_json::from_value::<ProjectionSet>(malformed) {
            Ok(_) => panic!("unknown Muskingum reference unexpectedly decoded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains(&expected.to_string()));
    }
    for (path, referenced) in [
        (incoming_reference, "z-incoming-previous"),
        (outgoing_reference, "a-outgoing-previous"),
    ] {
        let mut malformed = set_wire.clone();
        let mut cursor = &mut malformed;
        for component in path {
            cursor = if let Ok(index) = component.parse::<usize>() {
                &mut cursor[index]
            } else {
                &mut cursor[*component]
            };
        }
        cursor["value_kind"] = serde_json::json!("scalar");
        let expected = ProjectionError::ProjectionReferenceKind {
            owner: projection_id("muskingum-outgoing"),
            reference: projection_id(referenced),
            expected: ProjectionValueKind::Scalar,
            actual: ProjectionValueKind::Extensive,
        };
        let error = match serde_json::from_value::<ProjectionSet>(malformed) {
            Ok(_) => panic!("wrong-kind Muskingum reference unexpectedly decoded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains(&expected.to_string()));
    }
    let mut reversed = set_wire.clone();
    match reversed["specifications"].as_array_mut() {
        Some(specifications) => specifications.reverse(),
        None => panic!("serialized set specifications must be an array"),
    }
    let expected = ProjectionError::DependencyNotEarlier {
        owner: projection_id("muskingum-outgoing"),
        reference: projection_id("z-incoming-previous"),
        owner_index: 0,
        reference_index: 2,
    };
    let error = match serde_json::from_value::<ProjectionSet>(reversed) {
        Ok(_) => panic!("reversed Muskingum specifications unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains(&expected.to_string()));
    let mut custom = set_wire;
    custom["specifications"][2]["spec"] = serde_json::json!({"kind": "custom"});
    let error = match serde_json::from_value::<ProjectionSet>(custom) {
        Ok(_) => panic!("custom projection specification unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("unknown variant `custom`"));
}

#[test]
fn probe_recurrence_output_index_is_rejected() {
    let update = add(input("input-a"), parameter("parameter-a"));
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![RecurrenceInputBinding::new(
                InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(incoming()),
            )],
            vec![ParameterRef::new(
                parameter_id("parameter-a"),
                ExpressionValueKind::Scalar,
            )],
            vec![update],
            1,
        ),
        Err(ProjectionError::RecurrenceOutputIndex {
            projection: projection_id("rec-a"),
            state_len: 1,
            attempted: 1,
        })
    );
}

#[test]
fn probe_dependency_must_be_earlier_than_owner() {
    let projection_b = match lag(
        "projection-b",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-a"),
            ProjectionValueKind::Extensive,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("dependent lag fixture failed: {error}"),
    };
    let projection_a = match lag(
        "projection-a",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("authoritative lag fixture failed: {error}"),
    };
    let states = vec![
        state("projection-b", vec![InitialProjectionValue::Extensive(2.5)]),
        state(
            "projection-a",
            vec![InitialProjectionValue::Extensive(1.25)],
        ),
    ];
    assert_eq!(
        ProjectionSet::new(vec![projection_b, projection_a], states),
        Err(ProjectionError::DependencyNotEarlier {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-a"),
            owner_index: 0,
            reference_index: 1,
        })
    );
}

#[test]
fn probe_rolling_window_requires_window_minus_one_initial_values() {
    let rolling = match OrderedRollingAggregateSpec::new(
        R,
        S,
        projection_id("rolling-a"),
        ProjectionSource::AuthoritativeFact(incoming()),
        5,
        RollingAggregate::SumOldestToNewest,
    ) {
        Ok(value) => value,
        Err(error) => panic!("rolling fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![rolling.into()],
            vec![state(
                "rolling-a",
                vec![
                    InitialProjectionValue::Extensive(1.25),
                    InitialProjectionValue::Extensive(2.5),
                    InitialProjectionValue::Extensive(3.75),
                ],
            )],
        ),
        Err(ProjectionError::InitialStateLength {
            projection: projection_id("rolling-a"),
            expected: 4,
            actual: 3,
        })
    );
}

#[test]
fn probe_forbidden_recurrence_leaves_report_exact_authority() {
    let forcing = RuleExpr::forcing(R, S, ForcingRef::new(forcing_id("forcing-a")));
    let forcing_update = add(forcing, input("input-a"));
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![RecurrenceInputBinding::new(
                InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(incoming()),
            )],
            vec![],
            vec![forcing_update],
            0,
        ),
        Err(ProjectionError::ForbiddenRecurrenceLeaf {
            projection: projection_id("rec-a"),
            leaf: ForbiddenRecurrenceLeaf::Forcing,
        })
    );

    let table_update = match RuleExpr::interpolated_table(
        InterpolatedTableRef::new(table_id("table-a")),
        input("input-a"),
    ) {
        Ok(value) => value,
        Err(error) => panic!("interpolated table fixture failed: {error}"),
    };
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![RecurrenceInputBinding::new(
                InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(incoming()),
            )],
            vec![],
            vec![table_update],
            0,
        ),
        Err(ProjectionError::ForbiddenRecurrenceLeaf {
            projection: projection_id("rec-a"),
            leaf: ForbiddenRecurrenceLeaf::InterpolatedTable,
        })
    );
}

#[test]
fn probe_previous_state_bindings_reject_wrong_index_and_kind() {
    let bindings = |index, value_kind| {
        vec![
            RecurrenceInputBinding::new(
                InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(incoming()),
            ),
            RecurrenceInputBinding::new(
                InputRef::new(input_id("previous-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::PreviousState { index, value_kind },
            ),
        ]
    };
    let update = || add(input("input-a"), input("previous-a"));
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            bindings(0, ProjectionValueKind::Scalar),
            vec![],
            vec![update()],
            0,
        ),
        Err(ProjectionError::PreviousStateKind {
            projection: projection_id("rec-a"),
            input: input_id("previous-a"),
            index: 0,
            expected: ProjectionValueKind::Extensive,
            actual: ProjectionValueKind::Scalar,
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            bindings(1, ProjectionValueKind::Extensive),
            vec![],
            vec![update()],
            0,
        ),
        Err(ProjectionError::PreviousStateIndex {
            projection: projection_id("rec-a"),
            input: input_id("previous-a"),
            state_len: 1,
            attempted: 1,
        })
    );
}

#[test]
fn probe_initial_state_slot_kinds_are_checked() {
    let lag_a = match lag("lag-a", ProjectionSource::AuthoritativeFact(incoming()), 2) {
        Ok(value) => value,
        Err(error) => panic!("lag fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![lag_a],
            vec![state(
                "lag-a",
                vec![
                    InitialProjectionValue::Extensive(1.25),
                    InitialProjectionValue::Scalar(2.5),
                ],
            )],
        ),
        Err(ProjectionError::InitialStateKind {
            projection: projection_id("lag-a"),
            index: 1,
            expected: ProjectionValueKind::Extensive,
            actual: ProjectionValueKind::Scalar,
        })
    );
}

#[test]
fn remaining_recurrence_errors_are_reported_as_complete_typed_values() {
    let scalar_input = || {
        RecurrenceInputBinding::new(
            InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::AuthoritativeFact(incoming()),
        )
    };
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![
                ProjectionValueKind::Extensive,
                ProjectionValueKind::Extensive
            ],
            vec![scalar_input()],
            vec![],
            vec![input("input-a")],
            0,
        ),
        Err(ProjectionError::RecurrenceUpdateCount {
            projection: projection_id("rec-a"),
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![
                scalar_input(),
                RecurrenceInputBinding::new(
                    InputRef::new(input_id("input-z"), ExpressionValueKind::Scalar),
                    RecurrenceInputSource::AuthoritativeFact(incoming()),
                ),
            ],
            vec![],
            vec![input("input-a")],
            0,
        ),
        Err(ProjectionError::MissingInputBinding {
            projection: projection_id("rec-a"),
            input: input_id("input-z"),
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![scalar_input(), scalar_input()],
            vec![],
            vec![input("input-a")],
            0,
        ),
        Err(ProjectionError::DuplicateInputBinding {
            projection: projection_id("rec-a"),
            input: input_id("input-a"),
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![],
            vec![],
            vec![input("input-z")],
            0,
        ),
        Err(ProjectionError::MissingInputBinding {
            projection: projection_id("rec-a"),
            input: input_id("input-z"),
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![scalar_input()],
            vec![
                ParameterRef::new(parameter_id("parameter-a"), ExpressionValueKind::Scalar),
                ParameterRef::new(parameter_id("parameter-a"), ExpressionValueKind::Scalar),
            ],
            vec![input("input-a")],
            0,
        ),
        Err(ProjectionError::DuplicateParameterDeclaration {
            projection: projection_id("rec-a"),
            parameter: parameter_id("parameter-a"),
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![],
            vec![],
            vec![parameter("parameter-z")],
            0,
        ),
        Err(ProjectionError::MissingParameterDeclaration {
            projection: projection_id("rec-a"),
            parameter: parameter_id("parameter-z"),
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![],
            vec![ParameterRef::new(
                parameter_id("parameter-a"),
                ExpressionValueKind::Truth,
            )],
            vec![parameter("parameter-a")],
            0,
        ),
        Err(ProjectionError::ParameterDeclarationKind {
            projection: projection_id("rec-a"),
            parameter: parameter_id("parameter-a"),
            expected: ExpressionValueKind::Scalar,
            actual: ExpressionValueKind::Truth,
        })
    );
    let compared =
        match RuleExpr::comparison(ScalarComparison::Equal, input("input-a"), input("input-a")) {
            Ok(value) => value,
            Err(error) => panic!("comparison fixture failed: {error}"),
        };
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![scalar_input()],
            vec![],
            vec![compared],
            0,
        ),
        Err(ProjectionError::RecurrenceUpdateKind {
            projection: projection_id("rec-a"),
            index: 0,
            expected: ProjectionValueKind::Extensive,
            actual: ExpressionValueKind::Truth,
        })
    );
}

#[test]
fn inert_declared_parameter_and_input_binding_kind_guards_are_observed() {
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![RecurrenceInputBinding::new(
                InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(incoming()),
            )],
            vec![ParameterRef::new(
                parameter_id("parameter-z"),
                ExpressionValueKind::Scalar,
            )],
            vec![input("input-a")],
            0,
        ),
        Err(ProjectionError::MissingParameterDeclaration {
            projection: projection_id("rec-a"),
            parameter: parameter_id("parameter-z"),
        })
    );

    let leaf = input("input-a");
    let update = match RuleExpr::comparison(ScalarComparison::Equal, leaf.clone(), leaf) {
        Ok(value) => value,
        Err(error) => panic!("comparison fixture failed: {error}"),
    };
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Truth],
            vec![RecurrenceInputBinding::new(
                InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::PreviousState {
                    index: 0,
                    value_kind: ProjectionValueKind::Truth,
                },
            )],
            vec![],
            vec![update],
            0,
        ),
        Err(ProjectionError::InputBindingKind {
            projection: projection_id("rec-a"),
            input: input_id("input-a"),
            expected: ExpressionValueKind::Scalar,
            actual: ExpressionValueKind::Truth,
        })
    );
}

#[test]
fn projection_set_reports_remaining_state_and_reference_failures() {
    let owner_a = match lag(
        "projection-a",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("lag fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![owner_a.clone()],
            vec![
                state(
                    "projection-a",
                    vec![InitialProjectionValue::Extensive(1.25)],
                ),
                state("projection-z", vec![]),
            ],
        ),
        Err(ProjectionError::UnknownInitialState {
            projection: projection_id("projection-z"),
        })
    );
    let unknown = match lag(
        "projection-b",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-z"),
            ProjectionValueKind::Extensive,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("lag fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![unknown],
            vec![state(
                "projection-b",
                vec![InitialProjectionValue::Extensive(2.5)],
            )],
        ),
        Err(ProjectionError::UnknownProjectionReference {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-z"),
        })
    );
    let owner_b = match lag(
        "projection-b",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-a"),
            ProjectionValueKind::Scalar,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("lag fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![owner_a, owner_b],
            vec![
                state(
                    "projection-a",
                    vec![InitialProjectionValue::Extensive(1.25)],
                ),
                state("projection-b", vec![InitialProjectionValue::Scalar(2.5)],),
            ],
        ),
        Err(ProjectionError::ProjectionReferenceKind {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-a"),
            expected: ProjectionValueKind::Scalar,
            actual: ProjectionValueKind::Extensive,
        })
    );
}

#[test]
fn recurrence_and_previous_state_shapes_use_the_plan_exact_cases() {
    let recurrence = match FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("rec-a"),
        vec![ProjectionValueKind::Extensive],
        vec![RecurrenceInputBinding::new(
            InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::AuthoritativeFact(incoming()),
        )],
        vec![],
        vec![input("input-a")],
        0,
    ) {
        Ok(value) => value,
        Err(error) => panic!("recurrence fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![recurrence.into()],
            vec![state(
                "rec-a",
                vec![
                    InitialProjectionValue::Extensive(1.25),
                    InitialProjectionValue::Extensive(2.5),
                ],
            )],
        ),
        Err(ProjectionError::InitialStateLength {
            projection: projection_id("rec-a"),
            expected: 1,
            actual: 2,
        })
    );
    let binding = |index, value_kind| {
        RecurrenceInputBinding::new(
            InputRef::new(input_id("previous-a"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::PreviousState { index, value_kind },
        )
    };
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![
                ProjectionValueKind::Extensive,
                ProjectionValueKind::Extensive
            ],
            vec![binding(2, ProjectionValueKind::Extensive)],
            vec![],
            vec![input("previous-a"), input("previous-a")],
            0,
        ),
        Err(ProjectionError::PreviousStateIndex {
            projection: projection_id("rec-a"),
            input: input_id("previous-a"),
            state_len: 2,
            attempted: 2,
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![
                ProjectionValueKind::Extensive,
                ProjectionValueKind::Extensive
            ],
            vec![binding(0, ProjectionValueKind::Scalar)],
            vec![],
            vec![input("previous-a"), input("previous-a")],
            0,
        ),
        Err(ProjectionError::PreviousStateKind {
            projection: projection_id("rec-a"),
            input: input_id("previous-a"),
            index: 0,
            expected: ProjectionValueKind::Extensive,
            actual: ProjectionValueKind::Scalar,
        })
    );
}

#[test]
fn malformed_projection_wires_are_rejected_by_constructor_reconstruction() {
    const LAG: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"lag-a","value_kind":"extensive","spec":{"kind":"bounded_lag","source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}},"steps":0}}"#;
    const ROLLING: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","id":"rolling-a","value_kind":"extensive","spec":{"kind":"ordered_rolling_aggregate","source":{"kind":"authoritative_fact","selector":{"kind":"incoming_transfer_amount","compartment":"reach-a","substance":"substance-a"}},"window":0,"aggregate":"sum_oldest_to_newest"}}"#;
    let expected = ProjectionError::NonPositiveBound {
        projection: projection_id("lag-a"),
        bound: ProjectionBound::BoundedLagSteps,
        attempted: 0,
    }
    .to_string();
    let error = match serde_json::from_str::<ProjectionSpec>(LAG) {
        Ok(_) => panic!("zero-step lag wire unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains(&expected));
    let expected = ProjectionError::NonPositiveBound {
        projection: projection_id("rolling-a"),
        bound: ProjectionBound::RollingWindow,
        attempted: 0,
    }
    .to_string();
    let error = match serde_json::from_str::<ProjectionSpec>(ROLLING) {
        Ok(_) => panic!("zero-window rolling wire unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains(&expected));

    let valid = match FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("rec-a"),
        vec![ProjectionValueKind::Extensive],
        vec![RecurrenceInputBinding::new(
            InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::AuthoritativeFact(incoming()),
        )],
        vec![],
        vec![input("input-a")],
        0,
    ) {
        Ok(value) => ProjectionSpec::from(value),
        Err(error) => panic!("recurrence wire fixture failed: {error}"),
    };
    let encoded = match serde_json::to_value(&valid) {
        Ok(value) => value,
        Err(error) => panic!("recurrence wire fixture did not serialize: {error}"),
    };
    let mut cases = Vec::new();
    let mut empty = encoded.clone();
    empty["spec"]["state_kinds"] = serde_json::json!([]);
    empty["spec"]["updates"] = serde_json::json!([]);
    cases.push((
        empty,
        ProjectionError::NonPositiveBound {
            projection: projection_id("rec-a"),
            bound: ProjectionBound::RecurrenceStateSize,
            attempted: 0,
        },
    ));
    let mut duplicate = encoded.clone();
    let first_binding = duplicate["spec"]["inputs"][0].clone();
    duplicate["spec"]["inputs"] = serde_json::json!([first_binding.clone(), first_binding]);
    cases.push((
        duplicate,
        ProjectionError::DuplicateInputBinding {
            projection: projection_id("rec-a"),
            input: input_id("input-a"),
        },
    ));
    let mut count = encoded.clone();
    count["spec"]["state_kinds"] = serde_json::json!(["extensive", "extensive"]);
    cases.push((
        count,
        ProjectionError::RecurrenceUpdateCount {
            projection: projection_id("rec-a"),
            expected: 2,
            actual: 1,
        },
    ));
    for (wire, expected) in cases {
        let error = match serde_json::from_value::<ProjectionSpec>(wire) {
            Ok(_) => panic!("malformed recurrence wire unexpectedly decoded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains(&expected.to_string()));
    }

    let mut callable = match serde_json::to_value(&valid) {
        Ok(value) => value,
        Err(error) => panic!("spec fixture did not serialize: {error}"),
    };
    callable["spec"]["callable"] = serde_json::json!("x");
    let error = match serde_json::from_value::<ProjectionSpec>(callable) {
        Ok(_) => panic!("callable projection specification unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("unknown field `callable`"));
    let mut custom = match serde_json::to_value(&valid) {
        Ok(value) => value,
        Err(error) => panic!("spec fixture did not serialize: {error}"),
    };
    custom["spec"] = serde_json::json!({"kind": "custom"});
    let error = match serde_json::from_value::<ProjectionSpec>(custom) {
        Ok(_) => panic!("custom projection specification unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("unknown variant `custom`"));
    let unordered = ROLLING.replace("sum_oldest_to_newest", "unordered_sum");
    let error = match serde_json::from_str::<ProjectionSpec>(&unordered) {
        Ok(_) => panic!("unordered rolling aggregate unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("unknown variant `unordered_sum`")
    );
    let error = match serde_json::from_str::<InitialProjectorState>(
        r#"{"projection":"lag-a","values":[{"kind":"extensive","value":1e999}]}"#,
    ) {
        Ok(_) => panic!("out-of-range numeric state unexpectedly decoded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("number out of range"));
}

#[test]
fn inert_declared_parameter_is_rejected() {
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![RecurrenceInputBinding::new(
                InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
                RecurrenceInputSource::AuthoritativeFact(incoming()),
            )],
            vec![ParameterRef::new(
                parameter_id("parameter-z"),
                ExpressionValueKind::Scalar,
            )],
            vec![input("input-a")],
            0,
        ),
        Err(ProjectionError::MissingParameterDeclaration {
            projection: projection_id("rec-a"),
            parameter: parameter_id("parameter-z"),
        })
    );
}

#[test]
fn scalar_input_leaf_bound_to_truth_source_is_rejected() {
    let rhs = match RuleExpr::literal(R, S, 1.0) {
        Ok(value) => value,
        Err(error) => panic!("literal fixture failed: {error}"),
    };
    let update = match RuleExpr::comparison(
        ScalarComparison::LessThan,
        RuleExpr::input(
            R,
            S,
            InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
        ),
        rhs,
    ) {
        Ok(value) => value,
        Err(error) => panic!("comparison fixture failed: {error}"),
    };
    let result = FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("rec-a"),
        vec![ProjectionValueKind::Truth],
        vec![RecurrenceInputBinding::new(
            InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
            RecurrenceInputSource::PreviousState {
                index: 0,
                value_kind: ProjectionValueKind::Truth,
            },
        )],
        Vec::new(),
        vec![update],
        0,
    );
    assert_eq!(
        result,
        Err(ProjectionError::InputBindingKind {
            projection: projection_id("rec-a"),
            input: input_id("input-a"),
            expected: ExpressionValueKind::Scalar,
            actual: ExpressionValueKind::Truth,
        })
    );
}

#[test]
fn projection_reference_kind_must_match_the_referenced_projection() {
    let first = match lag(
        "projection-a",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("source projection fixture failed: {error}"),
    };
    let states = vec![
        extensive_state("projection-a"),
        extensive_state("projection-b"),
    ];
    let direct = match lag(
        "projection-b",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-a"),
            ProjectionValueKind::Scalar,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("direct dependency fixture failed: {error}"),
    };
    let expected = ProjectionError::ProjectionReferenceKind {
        owner: projection_id("projection-b"),
        reference: projection_id("projection-a"),
        expected: ProjectionValueKind::Scalar,
        actual: ProjectionValueKind::Extensive,
    };
    assert_eq!(
        ProjectionSet::new(vec![first.clone(), direct], states.clone()),
        Err(expected.clone())
    );
    assert_eq!(
        ProjectionSet::new(
            vec![
                first,
                recurrence_dependency("projection-b", "projection-a", ProjectionValueKind::Scalar,),
            ],
            states,
        ),
        Err(expected)
    );
}

#[test]
fn both_dependency_entry_families_reject_unknown_kind_order_and_cycles() {
    let source_a = match lag(
        "projection-a",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("source fixture failed: {error}"),
    };
    let direct_unknown = match lag(
        "projection-a",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-z"),
            ProjectionValueKind::Scalar,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("unknown direct fixture failed: {error}"),
    };
    let unknown = ProjectionError::UnknownProjectionReference {
        owner: projection_id("projection-a"),
        reference: projection_id("projection-z"),
    };
    assert_eq!(
        ProjectionSet::new(vec![direct_unknown], vec![extensive_state("projection-a")]),
        Err(unknown.clone())
    );
    assert_eq!(
        ProjectionSet::new(
            vec![recurrence_dependency(
                "projection-a",
                "projection-z",
                ProjectionValueKind::Scalar,
            )],
            vec![extensive_state("projection-a")],
        ),
        Err(unknown)
    );

    let direct_late = match lag(
        "projection-b",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-a"),
            ProjectionValueKind::Extensive,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("late direct fixture failed: {error}"),
    };
    let order = ProjectionError::DependencyNotEarlier {
        owner: projection_id("projection-b"),
        reference: projection_id("projection-a"),
        owner_index: 0,
        reference_index: 1,
    };
    let states = vec![
        extensive_state("projection-a"),
        extensive_state("projection-b"),
    ];
    assert_eq!(
        ProjectionSet::new(vec![direct_late, source_a.clone()], states.clone()),
        Err(order.clone())
    );
    assert_eq!(
        ProjectionSet::new(
            vec![
                recurrence_dependency(
                    "projection-b",
                    "projection-a",
                    ProjectionValueKind::Extensive,
                ),
                source_a,
            ],
            states,
        ),
        Err(order)
    );

    let cycle = ProjectionError::CyclicDependency {
        cycle: vec![
            projection_id("projection-a"),
            projection_id("projection-b"),
            projection_id("projection-a"),
        ],
    };
    let direct_a = match lag(
        "projection-a",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-b"),
            ProjectionValueKind::Extensive,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("cycle fixture failed: {error}"),
    };
    let direct_b = match lag(
        "projection-b",
        ProjectionSource::Projection(ProjectionRef::new(
            projection_id("projection-a"),
            ProjectionValueKind::Extensive,
        )),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("cycle fixture failed: {error}"),
    };
    let states = vec![
        extensive_state("projection-a"),
        extensive_state("projection-b"),
    ];
    for specifications in [
        vec![direct_a.clone(), direct_b.clone()],
        vec![direct_b, direct_a],
    ] {
        assert_eq!(
            ProjectionSet::new(specifications, states.clone()),
            Err(cycle.clone())
        );
    }
    let recurrence_a = recurrence_dependency(
        "projection-a",
        "projection-b",
        ProjectionValueKind::Extensive,
    );
    let recurrence_b = recurrence_dependency(
        "projection-b",
        "projection-a",
        ProjectionValueKind::Extensive,
    );
    for specifications in [
        vec![recurrence_a.clone(), recurrence_b.clone()],
        vec![recurrence_b, recurrence_a],
    ] {
        assert_eq!(
            ProjectionSet::new(specifications, states.clone()),
            Err(cycle.clone())
        );
    }

    let direct_base = match ProjectionSet::new(
        vec![
            match lag(
                "projection-a",
                ProjectionSource::AuthoritativeFact(incoming()),
                1,
            ) {
                Ok(value) => value,
                Err(error) => panic!("direct serde source failed: {error}"),
            },
            match lag(
                "projection-b",
                ProjectionSource::Projection(ProjectionRef::new(
                    projection_id("projection-a"),
                    ProjectionValueKind::Extensive,
                )),
                1,
            ) {
                Ok(value) => value,
                Err(error) => panic!("direct serde owner failed: {error}"),
            },
        ],
        states.clone(),
    ) {
        Ok(value) => value,
        Err(error) => panic!("direct serde set failed: {error}"),
    };
    let direct_wire = match serde_json::to_value(&direct_base) {
        Ok(value) => value,
        Err(error) => panic!("direct serde set did not serialize: {error}"),
    };
    let assert_wire_error = |wire: serde_json::Value, expected: ProjectionError| {
        let error = match serde_json::from_value::<ProjectionSet>(wire) {
            Ok(_) => panic!("invalid dependency wire unexpectedly decoded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains(&expected.to_string()));
    };
    let mut direct_unknown_wire = direct_wire.clone();
    direct_unknown_wire["specifications"][1]["spec"]["source"]["reference"]["id"] =
        serde_json::json!("projection-z");
    assert_wire_error(
        direct_unknown_wire,
        ProjectionError::UnknownProjectionReference {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-z"),
        },
    );
    let mut direct_kind_wire = direct_wire.clone();
    direct_kind_wire["specifications"][1]["spec"]["source"]["reference"]["value_kind"] =
        serde_json::json!("scalar");
    assert_wire_error(
        direct_kind_wire,
        ProjectionError::ProjectionReferenceKind {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-a"),
            expected: ProjectionValueKind::Scalar,
            actual: ProjectionValueKind::Extensive,
        },
    );
    let mut direct_order_wire = direct_wire.clone();
    match direct_order_wire["specifications"].as_array_mut() {
        Some(value) => value.reverse(),
        None => panic!("direct specifications must be an array"),
    }
    assert_wire_error(
        direct_order_wire,
        ProjectionError::DependencyNotEarlier {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-a"),
            owner_index: 0,
            reference_index: 1,
        },
    );
    let mut direct_cycle_wire = direct_wire;
    direct_cycle_wire["specifications"][0]["spec"]["source"] = serde_json::json!({
        "kind": "projection",
        "reference": {"id": "projection-b", "value_kind": "extensive"}
    });
    assert_wire_error(direct_cycle_wire, cycle.clone());

    let recurrence_base = match ProjectionSet::new(
        vec![
            independent_recurrence("projection-a"),
            recurrence_dependency(
                "projection-b",
                "projection-a",
                ProjectionValueKind::Extensive,
            ),
        ],
        states,
    ) {
        Ok(value) => value,
        Err(error) => panic!("recurrence serde set failed: {error}"),
    };
    let recurrence_wire = match serde_json::to_value(&recurrence_base) {
        Ok(value) => value,
        Err(error) => panic!("recurrence serde set did not serialize: {error}"),
    };
    let reference_path = &[
        "specifications",
        "1",
        "spec",
        "updates",
        "0",
        "expression",
        "reference",
    ];
    let mutate_reference = |mut wire: serde_json::Value, field: &str, value: &str| {
        let mut cursor = &mut wire;
        for component in reference_path {
            cursor = if let Ok(index) = component.parse::<usize>() {
                &mut cursor[index]
            } else {
                &mut cursor[*component]
            };
        }
        cursor[field] = serde_json::json!(value);
        wire
    };
    assert_wire_error(
        mutate_reference(recurrence_wire.clone(), "id", "projection-z"),
        ProjectionError::UnknownProjectionReference {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-z"),
        },
    );
    assert_wire_error(
        mutate_reference(recurrence_wire.clone(), "value_kind", "scalar"),
        ProjectionError::ProjectionReferenceKind {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-a"),
            expected: ProjectionValueKind::Scalar,
            actual: ProjectionValueKind::Extensive,
        },
    );
    let mut recurrence_order_wire = recurrence_wire.clone();
    match recurrence_order_wire["specifications"].as_array_mut() {
        Some(value) => value.reverse(),
        None => panic!("recurrence specifications must be an array"),
    }
    assert_wire_error(
        recurrence_order_wire,
        ProjectionError::DependencyNotEarlier {
            owner: projection_id("projection-b"),
            reference: projection_id("projection-a"),
            owner_index: 0,
            reference_index: 1,
        },
    );
    let mut recurrence_cycle_wire = recurrence_wire;
    recurrence_cycle_wire["specifications"][0]["spec"]["updates"][0]["expression"] = serde_json::json!({
        "kind": "projection",
        "reference": {"id": "projection-b", "value_kind": "extensive"}
    });
    assert_wire_error(recurrence_cycle_wire, cycle);
}

#[test]
fn competing_invalidities_follow_the_complete_diagnostic_precedence() {
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive, ProjectionValueKind::Scalar],
            Vec::new(),
            Vec::new(),
            vec![
                RuleExpr::literal(R, S, 1.0)
                    .unwrap_or_else(|error| { panic!("literal fixture failed: {error}") })
            ],
            2,
        ),
        Err(ProjectionError::RecurrenceUpdateCount {
            projection: projection_id("rec-a"),
            expected: 2,
            actual: 1,
        })
    );
    let duplicate = RecurrenceInputBinding::new(
        InputRef::new(input_id("input-a"), ExpressionValueKind::Scalar),
        RecurrenceInputSource::AuthoritativeFact(incoming()),
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            vec![duplicate.clone(), duplicate],
            Vec::new(),
            vec![parameter("parameter-z")],
            0,
        ),
        Err(ProjectionError::DuplicateInputBinding {
            projection: projection_id("rec-a"),
            input: input_id("input-a"),
        })
    );
    let spec_a = match lag(
        "projection-a",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("precedence fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(vec![spec_a], vec![state("projection-z", Vec::new())],),
        Err(ProjectionError::MissingInitialState {
            projection: projection_id("projection-a"),
        })
    );
    let spec_z = match lag(
        "projection-z",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("precedence fixture failed: {error}"),
    };
    let spec_a = match lag(
        "projection-a",
        ProjectionSource::AuthoritativeFact(incoming()),
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("precedence fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(vec![spec_z, spec_a], Vec::new()),
        Err(ProjectionError::MissingInitialState {
            projection: projection_id("projection-z"),
        })
    );
    let lag_a = match lag("lag-a", ProjectionSource::AuthoritativeFact(incoming()), 3) {
        Ok(value) => value,
        Err(error) => panic!("precedence fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![lag_a],
            vec![state(
                "lag-a",
                vec![
                    InitialProjectionValue::Scalar(1.25),
                    InitialProjectionValue::Extensive(2.5),
                ],
            )],
        ),
        Err(ProjectionError::InitialStateLength {
            projection: projection_id("lag-a"),
            expected: 3,
            actual: 2,
        })
    );
    let lag_a = match lag("lag-a", ProjectionSource::AuthoritativeFact(incoming()), 2) {
        Ok(value) => value,
        Err(error) => panic!("precedence fixture failed: {error}"),
    };
    assert_eq!(
        ProjectionSet::new(
            vec![lag_a],
            vec![state(
                "lag-a",
                vec![
                    InitialProjectionValue::Scalar(1.25),
                    InitialProjectionValue::Truth(true),
                ],
            )],
        ),
        Err(ProjectionError::InitialStateKind {
            projection: projection_id("lag-a"),
            index: 0,
            expected: ProjectionValueKind::Extensive,
            actual: ProjectionValueKind::Scalar,
        })
    );
}

#[test]
fn projection_dependencies_preserve_first_tree_occurrence_not_identity_sort_order() {
    let update = add(projection("z-projection"), projection("a-projection"));
    let recurrence = match FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("owner"),
        vec![ProjectionValueKind::Extensive],
        Vec::new(),
        Vec::new(),
        vec![update],
        0,
    ) {
        Ok(value) => value,
        Err(error) => panic!("dependency order fixture failed: {error}"),
    };
    assert_eq!(
        recurrence
            .dependencies()
            .map(|reference| reference.id().as_str())
            .collect::<Vec<_>>(),
        ["z-projection", "a-projection"]
    );
}

#[test]
fn three_node_cycle_preserves_traversal_from_lexicographically_least_id() {
    let a = recurrence_dependency(
        "projection-a",
        "projection-c",
        ProjectionValueKind::Extensive,
    );
    let b = recurrence_dependency(
        "projection-b",
        "projection-a",
        ProjectionValueKind::Extensive,
    );
    let c = recurrence_dependency(
        "projection-c",
        "projection-b",
        ProjectionValueKind::Extensive,
    );
    let states = vec![
        extensive_state("projection-a"),
        extensive_state("projection-b"),
        extensive_state("projection-c"),
    ];
    let expected = ProjectionError::CyclicDependency {
        cycle: vec![
            projection_id("projection-a"),
            projection_id("projection-c"),
            projection_id("projection-b"),
            projection_id("projection-a"),
        ],
    };
    for specifications in [vec![a.clone(), b.clone(), c.clone()], vec![c, a, b]] {
        assert_eq!(
            ProjectionSet::new(specifications, states.clone()),
            Err(expected.clone())
        );
    }
}

#[test]
fn truth_recurrence_slot_rejects_scalar_update() {
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Truth],
            Vec::new(),
            vec![ParameterRef::new(
                parameter_id("parameter-a"),
                ExpressionValueKind::Scalar,
            )],
            vec![parameter("parameter-a")],
            0,
        ),
        Err(ProjectionError::RecurrenceUpdateKind {
            projection: projection_id("rec-a"),
            index: 0,
            expected: ProjectionValueKind::Truth,
            actual: ExpressionValueKind::Scalar,
        })
    );
}

#[test]
fn index_bound_errors_preserve_asymmetric_state_length_and_attempted_values() {
    let binding = RecurrenceInputBinding::new(
        InputRef::new(input_id("previous-a"), ExpressionValueKind::Scalar),
        RecurrenceInputSource::PreviousState {
            index: 3,
            value_kind: ProjectionValueKind::Extensive,
        },
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![
                ProjectionValueKind::Extensive,
                ProjectionValueKind::Extensive
            ],
            vec![binding],
            Vec::new(),
            vec![input("previous-a"), input("previous-a")],
            0,
        ),
        Err(ProjectionError::PreviousStateIndex {
            projection: projection_id("rec-a"),
            input: input_id("previous-a"),
            state_len: 2,
            attempted: 3,
        })
    );
    assert_eq!(
        FiniteRecurrenceSpec::new(
            R,
            S,
            projection_id("rec-a"),
            vec![ProjectionValueKind::Extensive],
            Vec::new(),
            Vec::new(),
            vec![
                RuleExpr::literal(R, S, 1.0)
                    .unwrap_or_else(|error| { panic!("literal fixture failed: {error}") })
            ],
            2,
        ),
        Err(ProjectionError::RecurrenceOutputIndex {
            projection: projection_id("rec-a"),
            state_len: 1,
            attempted: 2,
        })
    );
}

#[test]
fn recurrence_public_kind_is_the_output_slot_kind_not_the_first_slot() {
    let truth_update = RuleExpr::parameter(
        R,
        S,
        ParameterRef::new(parameter_id("truth-p"), ExpressionValueKind::Truth),
    );
    let spec = match FiniteRecurrenceSpec::new(
        R,
        S,
        projection_id("rec-a"),
        vec![ProjectionValueKind::Scalar, ProjectionValueKind::Truth],
        Vec::new(),
        vec![
            ParameterRef::new(parameter_id("scalar-p"), ExpressionValueKind::Scalar),
            ParameterRef::new(parameter_id("truth-p"), ExpressionValueKind::Truth),
        ],
        vec![parameter("scalar-p"), truth_update],
        1,
    ) {
        Ok(value) => value,
        Err(error) => panic!("output-slot fixture failed: {error}"),
    };
    assert_eq!(spec.value_kind(), ProjectionValueKind::Truth);
    assert_eq!(
        ProjectionSpec::from(spec).value_kind(),
        ProjectionValueKind::Truth
    );
}
