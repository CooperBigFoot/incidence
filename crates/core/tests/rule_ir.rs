//! Black-box construction and serde tests for the neutral rule and partition IR.

use incidence_core::numerical_semantics::{NumericalSemanticsVersion, ScalarComparison};
use incidence_core::partition_expression::{
    Fraction, FractionBranch, FractionError, PartitionExpr, PartitionExprError, PartitionExprView,
};
use incidence_core::rule_expression::{
    MAX_RULE_EXPR_DEPTH, RuleExpr, RuleExprError, RuleExprOperand, RuleExprOperation, RuleExprView,
};
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef,
    ParameterId, ParameterRef, ProjectionId, ProjectionRef, RuleReferenceIdentityError,
    RuleReferenceKind, TableId, TransferBranchId,
};
use incidence_core::versions::RuleIrVersion;

const R: RuleIrVersion = RuleIrVersion::V1;
const S: NumericalSemanticsVersion = NumericalSemanticsVersion::V1;

fn input_id(value: &str) -> InputId {
    match InputId::parse(value) {
        Ok(v) => v,
        Err(e) => panic!("invalid input fixture: {e}"),
    }
}
fn parameter_id(value: &str) -> ParameterId {
    match ParameterId::parse(value) {
        Ok(v) => v,
        Err(e) => panic!("invalid parameter fixture: {e}"),
    }
}
fn forcing_id(value: &str) -> ForcingId {
    match ForcingId::parse(value) {
        Ok(v) => v,
        Err(e) => panic!("invalid forcing fixture: {e}"),
    }
}
fn projection_id(value: &str) -> ProjectionId {
    match ProjectionId::parse(value) {
        Ok(v) => v,
        Err(e) => panic!("invalid projection fixture: {e}"),
    }
}
fn table_id(value: &str) -> TableId {
    match TableId::parse(value) {
        Ok(v) => v,
        Err(e) => panic!("invalid table fixture: {e}"),
    }
}
fn branch_id(value: &str) -> TransferBranchId {
    match TransferBranchId::parse(value) {
        Ok(v) => v,
        Err(e) => panic!("invalid branch fixture: {e}"),
    }
}
fn literal(value: f64) -> RuleExpr {
    match RuleExpr::literal(R, S, value) {
        Ok(v) => v,
        Err(e) => panic!("invalid literal fixture: {e}"),
    }
}
fn fraction(value: f64) -> Fraction {
    match Fraction::new(S, value) {
        Ok(v) => v,
        Err(e) => panic!("invalid fraction fixture: {e}"),
    }
}
fn scalar_input(value: &str) -> RuleExpr {
    RuleExpr::input(
        R,
        S,
        InputRef::new(input_id(value), ExpressionValueKind::Scalar),
    )
}
fn truth_input(value: &str) -> RuleExpr {
    RuleExpr::input(
        R,
        S,
        InputRef::new(input_id(value), ExpressionValueKind::Truth),
    )
}
fn add(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    match RuleExpr::add(lhs, rhs) {
        Ok(v) => v,
        Err(e) => panic!("invalid add fixture: {e}"),
    }
}
fn subtract(lhs: RuleExpr, rhs: RuleExpr) -> RuleExpr {
    match RuleExpr::subtract(lhs, rhs) {
        Ok(v) => v,
        Err(e) => panic!("invalid subtract fixture: {e}"),
    }
}
fn json<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(value) => value,
        Err(error) => panic!("authored value must serialize: {error}"),
    }
}
fn split(retained: f64, branches: Vec<(&str, f64)>) -> Result<PartitionExpr, PartitionExprError> {
    PartitionExpr::fixed_fraction_split(
        R,
        S,
        fraction(retained),
        branches
            .into_iter()
            .map(|(id, value)| FractionBranch::new(branch_id(id), fraction(value)))
            .collect(),
    )
}

#[test]
fn identifiers_enforce_the_exact_shared_grammar() {
    assert_eq!(
        InputId::parse(""),
        Err(RuleReferenceIdentityError::Empty {
            kind: RuleReferenceKind::Input
        })
    );
    assert_eq!(
        ParameterId::parse(""),
        Err(RuleReferenceIdentityError::Empty {
            kind: RuleReferenceKind::Parameter
        })
    );
    assert_eq!(
        ForcingId::parse(""),
        Err(RuleReferenceIdentityError::Empty {
            kind: RuleReferenceKind::Forcing
        })
    );
    assert_eq!(
        ProjectionId::parse(""),
        Err(RuleReferenceIdentityError::Empty {
            kind: RuleReferenceKind::Projection
        })
    );
    assert_eq!(
        TableId::parse(""),
        Err(RuleReferenceIdentityError::Empty {
            kind: RuleReferenceKind::Table
        })
    );
    assert_eq!(
        TransferBranchId::parse(""),
        Err(RuleReferenceIdentityError::Empty {
            kind: RuleReferenceKind::TransferBranch
        })
    );
    assert_eq!(
        InputId::parse("1input"),
        Err(RuleReferenceIdentityError::InvalidStart {
            kind: RuleReferenceKind::Input,
            value: "1input".to_owned()
        })
    );
    assert_eq!(
        TableId::parse("table a"),
        Err(RuleReferenceIdentityError::InvalidCharacter {
            kind: RuleReferenceKind::Table,
            value: "table a".to_owned(),
            byte_index: 5,
            character: ' '
        })
    );
    assert_eq!(input_id("input-a_2").as_str(), "input-a_2");
}

#[test]
fn literal_and_fraction_bits_are_checked_and_canonical() {
    for (value, bits) in [
        (f64::NAN, 0x7ff8_0000_0000_0000),
        (f64::INFINITY, 0x7ff0_0000_0000_0000),
        (f64::NEG_INFINITY, 0xfff0_0000_0000_0000),
    ] {
        assert_eq!(
            RuleExpr::literal(R, S, value),
            Err(RuleExprError::NonFiniteLiteral { bits })
        );
    }
    assert_eq!(literal(-0.0).view().literal_bits(), 0);
    for (value, bits) in [
        (-0.125, 0xbfc0_0000_0000_0000),
        (1.125, 0x3ff2_0000_0000_0000),
        (f64::NAN, 0x7ff8_0000_0000_0000),
        (f64::INFINITY, 0x7ff0_0000_0000_0000),
        (f64::NEG_INFINITY, 0xfff0_0000_0000_0000),
    ] {
        assert_eq!(
            Fraction::new(S, value),
            Err(FractionError::OutOfRange { bits })
        );
    }
    assert_eq!(fraction(-0.0).bits(), 0);
    assert_eq!(fraction(0.0).bits(), 0);
    assert_eq!(fraction(1.0).bits(), 0x3ff0_0000_0000_0000);
}

trait LiteralView {
    fn literal_bits(self) -> u64;
}
impl LiteralView for incidence_core::rule_expression::RuleExprView<'_> {
    fn literal_bits(self) -> u64 {
        match self {
            Self::Literal(v) => v.bits(),
            _ => panic!("expected literal"),
        }
    }
}

#[test]
fn fixed_split_sorts_rejects_duplicates_and_requires_exact_total() {
    let duplicate = Err(PartitionExprError::DuplicateBranch {
        branch: branch_id("branch-a"),
    });
    assert_eq!(
        split(
            0.25,
            vec![("branch-a", 0.25), ("branch-b", 0.0), ("branch-a", 0.5)]
        ),
        duplicate
    );
    assert_eq!(
        split(
            0.25,
            vec![("branch-a", 0.5), ("branch-b", 0.0), ("branch-a", 0.25)]
        ),
        Err(PartitionExprError::DuplicateBranch {
            branch: branch_id("branch-a")
        })
    );
    assert_eq!(
        split(0.125, vec![("branch-a", 0.25), ("branch-b", 0.5)]),
        Err(PartitionExprError::InvalidFixedFractionTotal {
            total_bits: 0x3fec_0000_0000_0000
        })
    );
    assert!(split(1.0, vec![]).is_ok());
    assert_eq!(
        split(0.5, vec![]),
        Err(PartitionExprError::InvalidFixedFractionTotal {
            total_bits: 0x3fe0_0000_0000_0000
        })
    );
}

#[test]
fn fixed_split_accumulates_retained_before_sorted_transfer_fractions() {
    match split(0.1, vec![("branch-b", 0.7), ("branch-a", 0.2)]) {
        Ok(value) => assert_eq!(value.rule_ir_version(), RuleIrVersion::V1),
        Err(error) => panic!("retained-first accumulation must total exactly one: {error}"),
    }
}

fn incompatible(
    operation: RuleExprOperation,
    position: RuleExprOperand,
    expected: ExpressionValueKind,
    actual: ExpressionValueKind,
) -> RuleExprError {
    RuleExprError::IncompatibleValueKind {
        operation,
        position,
        expected,
        actual,
    }
}

#[test]
fn every_operand_position_rejects_the_complete_wrong_kind() {
    let truth = || truth_input("truth-a");
    let scalar = || literal(1.25);
    let binaries: [(
        RuleExprOperation,
        fn(RuleExpr, RuleExpr) -> Result<RuleExpr, RuleExprError>,
    ); 6] = [
        (RuleExprOperation::Add, RuleExpr::add),
        (RuleExprOperation::Subtract, RuleExpr::subtract),
        (RuleExprOperation::Multiply, RuleExpr::multiply),
        (RuleExprOperation::Divide, RuleExpr::divide),
        (RuleExprOperation::Minimum, RuleExpr::minimum),
        (RuleExprOperation::Maximum, RuleExpr::maximum),
    ];
    for (operation, constructor) in binaries {
        assert_eq!(
            constructor(truth(), scalar()),
            Err(incompatible(
                operation,
                RuleExprOperand::Left,
                ExpressionValueKind::Scalar,
                ExpressionValueKind::Truth
            ))
        );
        assert_eq!(
            constructor(scalar(), truth()),
            Err(incompatible(
                operation,
                RuleExprOperand::Right,
                ExpressionValueKind::Scalar,
                ExpressionValueKind::Truth
            ))
        );
    }
    assert_eq!(
        RuleExpr::clamp(truth(), scalar(), scalar()),
        Err(incompatible(
            RuleExprOperation::Clamp,
            RuleExprOperand::Value,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::clamp(scalar(), truth(), scalar()),
        Err(incompatible(
            RuleExprOperation::Clamp,
            RuleExprOperand::Lower,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::clamp(scalar(), scalar(), truth()),
        Err(incompatible(
            RuleExprOperation::Clamp,
            RuleExprOperand::Upper,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::comparison(ScalarComparison::Equal, truth(), scalar()),
        Err(incompatible(
            RuleExprOperation::Comparison,
            RuleExprOperand::Left,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::comparison(ScalarComparison::Equal, scalar(), truth()),
        Err(incompatible(
            RuleExprOperation::Comparison,
            RuleExprOperand::Right,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::select(scalar(), scalar(), scalar()),
        Err(incompatible(
            RuleExprOperation::Select,
            RuleExprOperand::Condition,
            ExpressionValueKind::Truth,
            ExpressionValueKind::Scalar
        ))
    );
    assert_eq!(
        RuleExpr::select(truth(), truth(), scalar()),
        Err(incompatible(
            RuleExprOperation::Select,
            RuleExprOperand::WhenTrue,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::select(truth(), scalar(), truth()),
        Err(incompatible(
            RuleExprOperation::Select,
            RuleExprOperand::WhenFalse,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::interpolated_table(InterpolatedTableRef::new(table_id("table-a")), truth()),
        Err(incompatible(
            RuleExprOperation::InterpolatedTable,
            RuleExprOperand::Input,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
}

fn depth_128() -> RuleExpr {
    let mut tree = literal(1.25);
    for index in 1..MAX_RULE_EXPR_DEPTH {
        tree = if index % 2 == 0 {
            add(tree, literal(2.5))
        } else {
            subtract(tree, literal(3.75))
        };
    }
    tree
}

#[test]
fn construction_and_deserialization_enforce_depth_128() {
    let tree = depth_128();
    assert_eq!(tree.depth(), 128);
    assert_eq!(
        RuleExpr::add(tree.clone(), literal(2.5)),
        Err(RuleExprError::ExpressionTooDeep {
            maximum: 128,
            attempted: 129
        })
    );
    let mut value = match serde_json::to_value(&tree) {
        Ok(v) => v,
        Err(e) => panic!("depth fixture must serialize: {e}"),
    };
    let prior = match value.get_mut("expression") {
        Some(slot) => std::mem::take(slot),
        None => panic!("serialized tree lacks expression"),
    };
    value["expression"] =
        serde_json::json!({"kind":"add","lhs":prior,"rhs":{"kind":"literal","value":2.5}});
    assert!(serde_json::from_value::<RuleExpr>(value).is_err());
}

#[test]
fn depth_uses_the_deepest_child_in_every_operand_position() {
    let deep = depth_128();
    assert_eq!(deep.depth(), 128);
    assert_eq!(
        RuleExpr::clamp(literal(1.25), deep.clone(), literal(2.5)),
        Err(RuleExprError::ExpressionTooDeep {
            maximum: 128,
            attempted: 129
        })
    );
    assert_eq!(
        RuleExpr::add(literal(1.25), deep.clone()),
        Err(RuleExprError::ExpressionTooDeep {
            maximum: 128,
            attempted: 129
        })
    );
    let condition =
        match RuleExpr::comparison(ScalarComparison::GreaterThan, literal(1.25), literal(2.5)) {
            Ok(value) => value,
            Err(error) => panic!("condition fixture failed: {error}"),
        };
    assert_eq!(
        RuleExpr::select(condition, literal(1.25), deep),
        Err(RuleExprError::ExpressionTooDeep {
            maximum: 128,
            attempted: 129
        })
    );
}

#[test]
fn views_expose_every_node_and_child_position_faithfully() {
    match scalar_input("input-a").view() {
        RuleExprView::Input(reference) => {
            assert_eq!(reference.id().as_str(), "input-a");
            assert_eq!(reference.value_kind(), ExpressionValueKind::Scalar);
        }
        _ => panic!("expected an input view"),
    }
    match RuleExpr::parameter(
        R,
        S,
        ParameterRef::new(parameter_id("parameter-a"), ExpressionValueKind::Scalar),
    )
    .view()
    {
        RuleExprView::Parameter(reference) => {
            assert_eq!(reference.id().as_str(), "parameter-a");
            assert_eq!(reference.value_kind(), ExpressionValueKind::Scalar);
        }
        _ => panic!("expected a parameter view"),
    }
    match RuleExpr::forcing(R, S, ForcingRef::new(forcing_id("forcing-a"))).view() {
        RuleExprView::Forcing(reference) => assert_eq!(reference.id().as_str(), "forcing-a"),
        _ => panic!("expected a forcing view"),
    }
    match RuleExpr::projection(
        R,
        S,
        ProjectionRef::new(projection_id("projection-a"), ExpressionValueKind::Scalar),
    )
    .view()
    {
        RuleExprView::Projection(reference) => {
            assert_eq!(reference.id().as_str(), "projection-a");
            assert_eq!(reference.value_kind(), ExpressionValueKind::Scalar);
        }
        _ => panic!("expected a projection view"),
    }
    assert_eq!(literal(1.25).view().literal_bits(), 0x3ff4_0000_0000_0000);

    let binaries = [
        RuleExpr::add(literal(1.25), literal(2.5)),
        RuleExpr::subtract(literal(1.25), literal(2.5)),
        RuleExpr::multiply(literal(1.25), literal(2.5)),
        RuleExpr::divide(literal(1.25), literal(2.5)),
        RuleExpr::minimum(literal(1.25), literal(2.5)),
        RuleExpr::maximum(literal(1.25), literal(2.5)),
    ];
    for (index, binary) in binaries.into_iter().enumerate() {
        let binary = match binary {
            Ok(value) => value,
            Err(error) => panic!("binary fixture failed: {error}"),
        };
        let (lhs, rhs) = match (index, binary.view()) {
            (0, RuleExprView::Add { lhs, rhs })
            | (1, RuleExprView::Subtract { lhs, rhs })
            | (2, RuleExprView::Multiply { lhs, rhs })
            | (3, RuleExprView::Divide { lhs, rhs })
            | (4, RuleExprView::Minimum { lhs, rhs })
            | (5, RuleExprView::Maximum { lhs, rhs }) => (lhs, rhs),
            _ => panic!("expected the matching binary view"),
        };
        assert_eq!(lhs.view().literal_bits(), 0x3ff4_0000_0000_0000);
        assert_eq!(rhs.view().literal_bits(), 0x4004_0000_0000_0000);
    }

    let clamp = match RuleExpr::clamp(literal(2.5), literal(1.25), literal(3.75)) {
        Ok(value) => value,
        Err(error) => panic!("clamp fixture failed: {error}"),
    };
    match clamp.view() {
        RuleExprView::Clamp {
            value,
            lower,
            upper,
        } => {
            assert_eq!(value.view().literal_bits(), 0x4004_0000_0000_0000);
            assert_eq!(lower.view().literal_bits(), 0x3ff4_0000_0000_0000);
            assert_eq!(upper.view().literal_bits(), 0x400e_0000_0000_0000);
        }
        _ => panic!("expected a clamp view"),
    }
    let comparison =
        match RuleExpr::comparison(ScalarComparison::GreaterThan, literal(1.25), literal(2.5)) {
            Ok(value) => value,
            Err(error) => panic!("comparison fixture failed: {error}"),
        };
    match comparison.view() {
        RuleExprView::Comparison {
            comparison,
            lhs,
            rhs,
        } => {
            assert_eq!(comparison, ScalarComparison::GreaterThan);
            assert_eq!(lhs.view().literal_bits(), 0x3ff4_0000_0000_0000);
            assert_eq!(rhs.view().literal_bits(), 0x4004_0000_0000_0000);
        }
        _ => panic!("expected a comparison view"),
    }
    let select = match RuleExpr::select(truth_input("truth-a"), literal(1.25), literal(2.5)) {
        Ok(value) => value,
        Err(error) => panic!("select fixture failed: {error}"),
    };
    match select.view() {
        RuleExprView::Select {
            condition,
            when_true,
            when_false,
        } => {
            match condition.view() {
                RuleExprView::Input(reference) => {
                    assert_eq!(reference.id().as_str(), "truth-a");
                    assert_eq!(reference.value_kind(), ExpressionValueKind::Truth);
                }
                _ => panic!("expected the select condition input"),
            }
            assert_eq!(when_true.view().literal_bits(), 0x3ff4_0000_0000_0000);
            assert_eq!(when_false.view().literal_bits(), 0x4004_0000_0000_0000);
        }
        _ => panic!("expected a select view"),
    }
    let table = match RuleExpr::interpolated_table(
        InterpolatedTableRef::new(table_id("table-a")),
        literal(1.25),
    ) {
        Ok(value) => value,
        Err(error) => panic!("table fixture failed: {error}"),
    };
    match table.view() {
        RuleExprView::InterpolatedTable { table, input } => {
            assert_eq!(table.id().as_str(), "table-a");
            assert_eq!(input.view().literal_bits(), 0x3ff4_0000_0000_0000);
        }
        _ => panic!("expected an interpolated-table view"),
    }

    match PartitionExpr::retain_all(R, S).view() {
        PartitionExprView::RetainAll => {}
        _ => panic!("expected a retain-all view"),
    }
    match PartitionExpr::release_all(R, S, branch_id("branch-a")).view() {
        PartitionExprView::ReleaseAll { branch } => {
            assert_eq!(branch.as_str(), "branch-a");
        }
        _ => panic!("expected a release-all view"),
    }
    let fixed = match split(0.125, vec![("branch-b", 0.5), ("branch-a", 0.375)]) {
        Ok(value) => value,
        Err(error) => panic!("fixed split fixture failed: {error}"),
    };
    match fixed.view() {
        PartitionExprView::FixedFractionSplit {
            retained_fraction,
            branches,
        } => {
            assert_eq!(retained_fraction.bits(), 0x3fc0_0000_0000_0000);
            assert_eq!(branches.len(), 2);
            assert_eq!(branches[0].branch().as_str(), "branch-a");
            assert_eq!(branches[0].fraction().bits(), 0x3fd8_0000_0000_0000);
            assert_eq!(branches[1].branch().as_str(), "branch-b");
            assert_eq!(branches[1].fraction().bits(), 0x3fe0_0000_0000_0000);
        }
        _ => panic!("expected a fixed-fraction-split view"),
    }
    match PartitionExpr::exogenous_series(
        R,
        S,
        branch_id("branch-a"),
        ForcingRef::new(forcing_id("forcing-a")),
    )
    .view()
    {
        PartitionExprView::ExogenousSeries { branch, series } => {
            assert_eq!(branch.as_str(), "branch-a");
            assert_eq!(series.id().as_str(), "forcing-a");
        }
        _ => panic!("expected an exogenous-series view"),
    }
    let constant = match PartitionExpr::constant_fraction_transfer(
        R,
        S,
        branch_id("branch-a"),
        fraction(0.375),
    ) {
        Ok(value) => value,
        Err(error) => panic!("constant fraction fixture failed: {error}"),
    };
    match constant.view() {
        PartitionExprView::ConstantFractionTransfer { branch, fraction } => {
            assert_eq!(branch.as_str(), "branch-a");
            assert_eq!(fraction.bits(), 0x3fd8_0000_0000_0000);
        }
        _ => panic!("expected a constant-fraction-transfer view"),
    }
}

fn representative_rule() -> RuleExpr {
    let projection = RuleExpr::projection(
        R,
        S,
        ProjectionRef::new(
            projection_id("projection-lag-a"),
            ExpressionValueKind::Scalar,
        ),
    );
    let condition =
        match RuleExpr::comparison(ScalarComparison::GreaterThan, projection, literal(1.25)) {
            Ok(v) => v,
            Err(e) => panic!("condition fixture failed: {e}"),
        };
    let forcing = RuleExpr::forcing(R, S, ForcingRef::new(forcing_id("forcing-a")));
    let parameter = RuleExpr::parameter(
        R,
        S,
        ParameterRef::new(parameter_id("parameter-a"), ExpressionValueKind::Scalar),
    );
    let table = match RuleExpr::interpolated_table(
        InterpolatedTableRef::new(table_id("table-a")),
        parameter,
    ) {
        Ok(v) => v,
        Err(e) => panic!("table fixture failed: {e}"),
    };
    let when_true = add(forcing, table);
    let when_false = scalar_input("input-a");
    match RuleExpr::select(condition, when_true, when_false) {
        Ok(v) => v,
        Err(e) => panic!("select fixture failed: {e}"),
    }
}

#[test]
fn representative_rule_has_the_exact_wire_contract() {
    const JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"select","condition":{"kind":"comparison","comparison":"greater_than","lhs":{"kind":"projection","reference":{"id":"projection-lag-a","value_kind":"scalar"}},"rhs":{"kind":"literal","value":1.25}},"when_true":{"kind":"add","lhs":{"kind":"forcing","reference":{"id":"forcing-a"}},"rhs":{"kind":"interpolated_table","table":{"id":"table-a"},"input":{"kind":"parameter","reference":{"id":"parameter-a","value_kind":"scalar"}}}},"when_false":{"kind":"input","reference":{"id":"input-a","value_kind":"scalar"}}}}"#;
    let tree = representative_rule();
    assert_eq!(json(&tree), JSON);
    let decoded: RuleExpr = match serde_json::from_str(JSON) {
        Ok(v) => v,
        Err(e) => panic!("rule JSON must decode: {e}"),
    };
    assert_eq!(decoded, tree);
    assert_eq!(json(&decoded), JSON);
}

#[test]
fn representative_partition_sorts_and_has_the_exact_wire_contract() {
    const JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","partition":{"kind":"fixed_fraction_split","retained_fraction":0.125,"branches":[{"branch":"branch-a","fraction":0.375},{"branch":"branch-b","fraction":0.5}]}}"#;
    let left = match split(0.125, vec![("branch-b", 0.5), ("branch-a", 0.375)]) {
        Ok(v) => v,
        Err(e) => panic!("split failed: {e}"),
    };
    let right = match split(0.125, vec![("branch-a", 0.375), ("branch-b", 0.5)]) {
        Ok(v) => v,
        Err(e) => panic!("split failed: {e}"),
    };
    assert_eq!(left, right);
    assert_eq!(json(&left), JSON);
    let decoded: PartitionExpr = match serde_json::from_str(JSON) {
        Ok(v) => v,
        Err(e) => panic!("partition JSON failed: {e}"),
    };
    assert_eq!(decoded, left);
    assert_eq!(json(&decoded), JSON);
}

#[test]
fn malformed_wire_inputs_cannot_bypass_construction() {
    let rules = [
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"input","reference":{"id":"1input","value_kind":"scalar"}}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"add","lhs":{"kind":"input","reference":{"id":"truth-a","value_kind":"truth"}},"rhs":{"kind":"literal","value":1.25}}}"#,
        r#"{"rule_ir_version":"v2","numerical_semantics_version":"v1","expression":{"kind":"literal","value":1.25}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v2","expression":{"kind":"literal","value":1.25}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"input","reference":{"id":"input-a","value_kind":"number"}}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"comparison","comparison":"approximately_equal","lhs":{"kind":"literal","value":1.25},"rhs":{"kind":"literal","value":2.5}}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"literal","value":1.25,"opaque":true}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"custom","value":1.25}}"#,
    ];
    for json in rules {
        assert!(serde_json::from_str::<RuleExpr>(json).is_err());
    }
    let partitions = [
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","partition":{"kind":"fixed_fraction_split","retained_fraction":0.25,"branches":[{"branch":"branch-a","fraction":0.25},{"branch":"branch-a","fraction":0.5}]}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","partition":{"kind":"fixed_fraction_split","retained_fraction":0.125,"branches":[{"branch":"branch-a","fraction":0.25},{"branch":"branch-b","fraction":0.5}]}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","partition":{"kind":"constant_fraction_transfer","branch":"branch-a","fraction":1.125}}"#,
        r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","partition":{"kind":"custom"}}"#,
    ];
    for json in partitions {
        assert!(serde_json::from_str::<PartitionExpr>(json).is_err());
    }
}

#[test]
fn all_partition_shapes_round_trip() {
    let values = [
        PartitionExpr::retain_all(R, S),
        PartitionExpr::release_all(R, S, branch_id("branch-a")),
        match split(0.125, vec![("branch-a", 0.375), ("branch-b", 0.5)]) {
            Ok(v) => v,
            Err(e) => panic!("split failed: {e}"),
        },
        PartitionExpr::exogenous_series(
            R,
            S,
            branch_id("branch-a"),
            ForcingRef::new(forcing_id("forcing-a")),
        ),
        match PartitionExpr::constant_fraction_transfer(
            R,
            S,
            branch_id("branch-a"),
            fraction(0.375),
        ) {
            Ok(v) => v,
            Err(e) => panic!("constant fraction failed: {e}"),
        },
    ];
    for value in values {
        let json = match serde_json::to_string(&value) {
            Ok(v) => v,
            Err(e) => panic!("partition serialization failed: {e}"),
        };
        let decoded: PartitionExpr = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("partition deserialization failed: {e}"),
        };
        assert_eq!(decoded, value);
    }
    assert!(
        PartitionExpr::constant_fraction_transfer(R, S, branch_id("branch-a"), fraction(0.0))
            .is_ok()
    );
    assert!(
        PartitionExpr::constant_fraction_transfer(R, S, branch_id("branch-a"), fraction(1.0))
            .is_ok()
    );
}

#[test]
fn truth_valued_parameter_and_projection_expressions_report_truth() {
    let truth_parameter = RuleExpr::parameter(
        R,
        S,
        ParameterRef::new(parameter_id("parameter-a"), ExpressionValueKind::Truth),
    );
    let truth_projection = RuleExpr::projection(
        R,
        S,
        ProjectionRef::new(projection_id("projection-a"), ExpressionValueKind::Truth),
    );
    assert_eq!(
        RuleExpr::add(truth_parameter.clone(), literal(1.25)),
        Err(incompatible(
            RuleExprOperation::Add,
            RuleExprOperand::Left,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert_eq!(
        RuleExpr::add(truth_projection.clone(), literal(1.25)),
        Err(incompatible(
            RuleExprOperation::Add,
            RuleExprOperand::Left,
            ExpressionValueKind::Scalar,
            ExpressionValueKind::Truth
        ))
    );
    assert!(RuleExpr::select(truth_parameter, literal(1.25), literal(2.5)).is_ok());
    assert!(RuleExpr::select(truth_projection, literal(1.25), literal(2.5)).is_ok());
}

#[test]
fn clamp_wire_contract_pins_each_child_slot() {
    const CLAMP_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"clamp","value":{"kind":"literal","value":2.5},"lower":{"kind":"literal","value":1.25},"upper":{"kind":"literal","value":3.75}}}"#;
    let clamp = match RuleExpr::clamp(literal(2.5), literal(1.25), literal(3.75)) {
        Ok(value) => value,
        Err(error) => panic!("clamp fixture failed: {error}"),
    };
    assert_eq!(json(&clamp), CLAMP_JSON);
    let decoded: RuleExpr = match serde_json::from_str(CLAMP_JSON) {
        Ok(value) => value,
        Err(error) => panic!("clamp JSON must decode: {error}"),
    };
    assert_eq!(decoded, clamp);
}

#[test]
fn remaining_binary_wire_contracts_pin_kind_and_child_slots() {
    const SUBTRACT_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"subtract","lhs":{"kind":"literal","value":1.25},"rhs":{"kind":"literal","value":2.5}}}"#;
    const MULTIPLY_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"multiply","lhs":{"kind":"literal","value":1.25},"rhs":{"kind":"literal","value":2.5}}}"#;
    const DIVIDE_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"divide","lhs":{"kind":"literal","value":1.25},"rhs":{"kind":"literal","value":2.5}}}"#;
    const MINIMUM_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"minimum","lhs":{"kind":"literal","value":1.25},"rhs":{"kind":"literal","value":2.5}}}"#;
    const MAXIMUM_JSON: &str = r#"{"rule_ir_version":"v1","numerical_semantics_version":"v1","expression":{"kind":"maximum","lhs":{"kind":"literal","value":1.25},"rhs":{"kind":"literal","value":2.5}}}"#;

    let fixtures = [
        (
            RuleExpr::subtract(literal(1.25), literal(2.5)),
            SUBTRACT_JSON,
        ),
        (
            RuleExpr::multiply(literal(1.25), literal(2.5)),
            MULTIPLY_JSON,
        ),
        (RuleExpr::divide(literal(1.25), literal(2.5)), DIVIDE_JSON),
        (RuleExpr::minimum(literal(1.25), literal(2.5)), MINIMUM_JSON),
        (RuleExpr::maximum(literal(1.25), literal(2.5)), MAXIMUM_JSON),
    ];
    for (fixture, expected_json) in fixtures {
        let expression = match fixture {
            Ok(value) => value,
            Err(error) => panic!("binary fixture failed: {error}"),
        };
        assert_eq!(json(&expression), expected_json);
        let decoded: RuleExpr = match serde_json::from_str(expected_json) {
            Ok(value) => value,
            Err(error) => panic!("binary JSON must decode: {error}"),
        };
        assert_eq!(decoded, expression);
    }
}
