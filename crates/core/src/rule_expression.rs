//! rule_expression : VersionedTypedLeaves × ClosedScalarTruthCombinators ⇀ RuleExpr   (pure, deterministic)

use crate::canonical_encoding::{
    CanonicalEncode, CanonicalEncodingError, CanonicalField, CanonicalPayloadWriter,
};
use crate::numerical_semantics::{NumericalSemanticsVersion, ScalarComparison};
use crate::rule_reference::{
    ExpressionValueKind, ForcingRef, InputRef, InterpolatedTableRef, ParameterRef, ProjectionRef,
    ProjectionValueKind,
};
use crate::versions::RuleIrVersion;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const MAX_RULE_EXPR_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiniteLiteral {
    semantics: NumericalSemanticsVersion,
    bits: u64,
}
impl FiniteLiteral {
    /// Creates a finite literal under the selected semantics.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError::NonFiniteLiteral`] when `value` is NaN or infinite.
    pub fn new(semantics: NumericalSemanticsVersion, value: f64) -> Result<Self, RuleExprError> {
        let normalized =
            semantics
                .normalize(value)
                .map_err(|_| RuleExprError::NonFiniteLiteral {
                    bits: value.to_bits(),
                })?;
        Ok(Self {
            semantics,
            bits: normalized.to_bits(),
        })
    }
    pub fn numerical_semantics_version(self) -> NumericalSemanticsVersion {
        self.semantics
    }
    pub fn value(self) -> f64 {
        f64::from_bits(self.bits)
    }
    pub fn bits(self) -> u64 {
        self.bits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleExprOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Minimum,
    Maximum,
    Clamp,
    Comparison,
    Select,
    InterpolatedTable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleExprOperand {
    Left,
    Right,
    Value,
    Lower,
    Upper,
    Condition,
    WhenTrue,
    WhenFalse,
    Input,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuleExprError {
    /// Fires when a literal is NaN or infinite.
    #[error("rule literal rejects non-finite bits {bits:#018x}")]
    NonFiniteLiteral { bits: u64 },
    /// Fires when an operation receives an operand of the wrong expression value kind.
    #[error("{operation:?} operand {position:?} expected {expected:?}, got {actual:?}")]
    IncompatibleValueKind {
        operation: RuleExprOperation,
        position: RuleExprOperand,
        expected: ExpressionValueKind,
        actual: ExpressionValueKind,
    },
    /// Fires when constructing an expression would exceed the maximum checked depth.
    #[error("rule expression depth {attempted} exceeds maximum {maximum}")]
    ExpressionTooDeep { maximum: usize, attempted: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RuleNode {
    Input(InputRef),
    Parameter(ParameterRef),
    Forcing(ForcingRef),
    Projection(ProjectionRef),
    Literal(FiniteLiteral),
    Add(Box<RuleExpr>, Box<RuleExpr>),
    Subtract(Box<RuleExpr>, Box<RuleExpr>),
    Multiply(Box<RuleExpr>, Box<RuleExpr>),
    Divide(Box<RuleExpr>, Box<RuleExpr>),
    Minimum(Box<RuleExpr>, Box<RuleExpr>),
    Maximum(Box<RuleExpr>, Box<RuleExpr>),
    Clamp {
        value: Box<RuleExpr>,
        lower: Box<RuleExpr>,
        upper: Box<RuleExpr>,
    },
    Comparison {
        comparison: ScalarComparison,
        lhs: Box<RuleExpr>,
        rhs: Box<RuleExpr>,
    },
    Select {
        condition: Box<RuleExpr>,
        when_true: Box<RuleExpr>,
        when_false: Box<RuleExpr>,
    },
    InterpolatedTable {
        table: InterpolatedTableRef,
        input: Box<RuleExpr>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleExpr {
    rule_ir: RuleIrVersion,
    semantics: NumericalSemanticsVersion,
    node: RuleNode,
    depth: usize,
}

pub enum RuleExprView<'a> {
    Input(&'a InputRef),
    Parameter(&'a ParameterRef),
    Forcing(&'a ForcingRef),
    Projection(&'a ProjectionRef),
    Literal(FiniteLiteral),
    Add {
        lhs: &'a RuleExpr,
        rhs: &'a RuleExpr,
    },
    Subtract {
        lhs: &'a RuleExpr,
        rhs: &'a RuleExpr,
    },
    Multiply {
        lhs: &'a RuleExpr,
        rhs: &'a RuleExpr,
    },
    Divide {
        lhs: &'a RuleExpr,
        rhs: &'a RuleExpr,
    },
    Minimum {
        lhs: &'a RuleExpr,
        rhs: &'a RuleExpr,
    },
    Maximum {
        lhs: &'a RuleExpr,
        rhs: &'a RuleExpr,
    },
    Clamp {
        value: &'a RuleExpr,
        lower: &'a RuleExpr,
        upper: &'a RuleExpr,
    },
    Comparison {
        comparison: ScalarComparison,
        lhs: &'a RuleExpr,
        rhs: &'a RuleExpr,
    },
    Select {
        condition: &'a RuleExpr,
        when_true: &'a RuleExpr,
        when_false: &'a RuleExpr,
    },
    InterpolatedTable {
        table: &'a InterpolatedTableRef,
        input: &'a RuleExpr,
    },
}

impl RuleExpr {
    fn leaf(rule_ir: RuleIrVersion, semantics: NumericalSemanticsVersion, node: RuleNode) -> Self {
        Self {
            rule_ir,
            semantics,
            node,
            depth: 1,
        }
    }
    pub fn input(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        reference: InputRef,
    ) -> Self {
        Self::leaf(rule_ir, semantics, RuleNode::Input(reference))
    }
    pub fn parameter(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        reference: ParameterRef,
    ) -> Self {
        Self::leaf(rule_ir, semantics, RuleNode::Parameter(reference))
    }
    pub fn forcing(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        reference: ForcingRef,
    ) -> Self {
        Self::leaf(rule_ir, semantics, RuleNode::Forcing(reference))
    }
    pub fn projection(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        reference: ProjectionRef,
    ) -> Self {
        Self::leaf(rule_ir, semantics, RuleNode::Projection(reference))
    }
    pub fn literal(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        value: f64,
    ) -> Result<Self, RuleExprError> {
        Ok(Self::leaf(
            rule_ir,
            semantics,
            RuleNode::Literal(FiniteLiteral::new(semantics, value)?),
        ))
    }
    fn require(
        expr: &Self,
        operation: RuleExprOperation,
        position: RuleExprOperand,
        expected: ExpressionValueKind,
    ) -> Result<(), RuleExprError> {
        let actual = expr.value_kind();
        if actual == expected {
            Ok(())
        } else {
            Err(RuleExprError::IncompatibleValueKind {
                operation,
                position,
                expected,
                actual,
            })
        }
    }
    fn composite_depth(first: &Self, remaining: &[&Self]) -> Result<usize, RuleExprError> {
        let maximum_child = remaining
            .iter()
            .fold(first.depth, |depth, child| depth.max(child.depth));
        let attempted = maximum_child
            .checked_add(1)
            .ok_or(RuleExprError::ExpressionTooDeep {
                maximum: MAX_RULE_EXPR_DEPTH,
                attempted: usize::MAX,
            })?;
        if attempted > MAX_RULE_EXPR_DEPTH {
            Err(RuleExprError::ExpressionTooDeep {
                maximum: MAX_RULE_EXPR_DEPTH,
                attempted,
            })
        } else {
            Ok(attempted)
        }
    }
    fn binary(
        lhs: Self,
        rhs: Self,
        operation: RuleExprOperation,
        node: fn(Box<Self>, Box<Self>) -> RuleNode,
    ) -> Result<Self, RuleExprError> {
        Self::require(
            &lhs,
            operation,
            RuleExprOperand::Left,
            ExpressionValueKind::Scalar,
        )?;
        Self::require(
            &rhs,
            operation,
            RuleExprOperand::Right,
            ExpressionValueKind::Scalar,
        )?;
        let depth = Self::composite_depth(&lhs, &[&rhs])?;
        Ok(Self {
            rule_ir: lhs.rule_ir,
            semantics: lhs.semantics,
            node: node(Box::new(lhs), Box::new(rhs)),
            depth,
        })
    }
    /// Creates an ordered scalar addition expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn add(lhs: Self, rhs: Self) -> Result<Self, RuleExprError> {
        Self::binary(lhs, rhs, RuleExprOperation::Add, RuleNode::Add)
    }
    /// Creates an ordered scalar subtraction expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn subtract(lhs: Self, rhs: Self) -> Result<Self, RuleExprError> {
        Self::binary(lhs, rhs, RuleExprOperation::Subtract, RuleNode::Subtract)
    }
    /// Creates an ordered scalar multiplication expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn multiply(lhs: Self, rhs: Self) -> Result<Self, RuleExprError> {
        Self::binary(lhs, rhs, RuleExprOperation::Multiply, RuleNode::Multiply)
    }
    /// Creates an ordered scalar division expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn divide(lhs: Self, rhs: Self) -> Result<Self, RuleExprError> {
        Self::binary(lhs, rhs, RuleExprOperation::Divide, RuleNode::Divide)
    }
    /// Creates an ordered scalar minimum expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn minimum(lhs: Self, rhs: Self) -> Result<Self, RuleExprError> {
        Self::binary(lhs, rhs, RuleExprOperation::Minimum, RuleNode::Minimum)
    }
    /// Creates an ordered scalar maximum expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn maximum(lhs: Self, rhs: Self) -> Result<Self, RuleExprError> {
        Self::binary(lhs, rhs, RuleExprOperation::Maximum, RuleNode::Maximum)
    }
    /// Creates an ordered scalar clamp expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn clamp(value: Self, lower: Self, upper: Self) -> Result<Self, RuleExprError> {
        for (expr, position) in [
            (&value, RuleExprOperand::Value),
            (&lower, RuleExprOperand::Lower),
            (&upper, RuleExprOperand::Upper),
        ] {
            Self::require(
                expr,
                RuleExprOperation::Clamp,
                position,
                ExpressionValueKind::Scalar,
            )?;
        }
        let depth = Self::composite_depth(&value, &[&lower, &upper])?;
        Ok(Self {
            rule_ir: value.rule_ir,
            semantics: value.semantics,
            node: RuleNode::Clamp {
                value: Box::new(value),
                lower: Box::new(lower),
                upper: Box::new(upper),
            },
            depth,
        })
    }
    /// Creates an ordered scalar comparison expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand is not scalar or the depth limit is exceeded.
    pub fn comparison(
        comparison: ScalarComparison,
        lhs: Self,
        rhs: Self,
    ) -> Result<Self, RuleExprError> {
        Self::require(
            &lhs,
            RuleExprOperation::Comparison,
            RuleExprOperand::Left,
            ExpressionValueKind::Scalar,
        )?;
        Self::require(
            &rhs,
            RuleExprOperation::Comparison,
            RuleExprOperand::Right,
            ExpressionValueKind::Scalar,
        )?;
        let depth = Self::composite_depth(&lhs, &[&rhs])?;
        Ok(Self {
            rule_ir: lhs.rule_ir,
            semantics: lhs.semantics,
            node: RuleNode::Comparison {
                comparison,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            depth,
        })
    }
    /// Creates a truth-directed scalar selection expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when an operand has the wrong kind or the depth limit is exceeded.
    pub fn select(
        condition: Self,
        when_true: Self,
        when_false: Self,
    ) -> Result<Self, RuleExprError> {
        Self::require(
            &condition,
            RuleExprOperation::Select,
            RuleExprOperand::Condition,
            ExpressionValueKind::Truth,
        )?;
        Self::require(
            &when_true,
            RuleExprOperation::Select,
            RuleExprOperand::WhenTrue,
            ExpressionValueKind::Scalar,
        )?;
        Self::require(
            &when_false,
            RuleExprOperation::Select,
            RuleExprOperand::WhenFalse,
            ExpressionValueKind::Scalar,
        )?;
        let depth = Self::composite_depth(&condition, &[&when_true, &when_false])?;
        Ok(Self {
            rule_ir: condition.rule_ir,
            semantics: condition.semantics,
            node: RuleNode::Select {
                condition: Box::new(condition),
                when_true: Box::new(when_true),
                when_false: Box::new(when_false),
            },
            depth,
        })
    }
    /// Creates a typed interpolation-table lookup expression.
    ///
    /// # Errors
    ///
    /// Returns [`RuleExprError`] when the input is not scalar or the depth limit is exceeded.
    pub fn interpolated_table(
        table: InterpolatedTableRef,
        input: Self,
    ) -> Result<Self, RuleExprError> {
        Self::require(
            &input,
            RuleExprOperation::InterpolatedTable,
            RuleExprOperand::Input,
            ExpressionValueKind::Scalar,
        )?;
        let depth = Self::composite_depth(&input, &[])?;
        Ok(Self {
            rule_ir: input.rule_ir,
            semantics: input.semantics,
            node: RuleNode::InterpolatedTable {
                table,
                input: Box::new(input),
            },
            depth,
        })
    }
    pub fn value_kind(&self) -> ExpressionValueKind {
        match self.node {
            RuleNode::Input(ref r) => r.value_kind(),
            RuleNode::Parameter(ref r) => r.value_kind(),
            RuleNode::Projection(ref r) => match r.value_kind() {
                ProjectionValueKind::Extensive | ProjectionValueKind::Scalar => {
                    ExpressionValueKind::Scalar
                }
                ProjectionValueKind::Truth => ExpressionValueKind::Truth,
            },
            RuleNode::Comparison { .. } => ExpressionValueKind::Truth,
            RuleNode::Forcing(_)
            | RuleNode::Literal(_)
            | RuleNode::Add(_, _)
            | RuleNode::Subtract(_, _)
            | RuleNode::Multiply(_, _)
            | RuleNode::Divide(_, _)
            | RuleNode::Minimum(_, _)
            | RuleNode::Maximum(_, _)
            | RuleNode::Clamp { .. }
            | RuleNode::Select { .. }
            | RuleNode::InterpolatedTable { .. } => ExpressionValueKind::Scalar,
        }
    }
    pub fn rule_ir_version(&self) -> RuleIrVersion {
        self.rule_ir
    }
    pub fn numerical_semantics_version(&self) -> NumericalSemanticsVersion {
        self.semantics
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn view(&self) -> RuleExprView<'_> {
        match &self.node {
            RuleNode::Input(r) => RuleExprView::Input(r),
            RuleNode::Parameter(r) => RuleExprView::Parameter(r),
            RuleNode::Forcing(r) => RuleExprView::Forcing(r),
            RuleNode::Projection(r) => RuleExprView::Projection(r),
            RuleNode::Literal(v) => RuleExprView::Literal(*v),
            RuleNode::Add(l, r) => RuleExprView::Add { lhs: l, rhs: r },
            RuleNode::Subtract(l, r) => RuleExprView::Subtract { lhs: l, rhs: r },
            RuleNode::Multiply(l, r) => RuleExprView::Multiply { lhs: l, rhs: r },
            RuleNode::Divide(l, r) => RuleExprView::Divide { lhs: l, rhs: r },
            RuleNode::Minimum(l, r) => RuleExprView::Minimum { lhs: l, rhs: r },
            RuleNode::Maximum(l, r) => RuleExprView::Maximum { lhs: l, rhs: r },
            RuleNode::Clamp {
                value,
                lower,
                upper,
            } => RuleExprView::Clamp {
                value,
                lower,
                upper,
            },
            RuleNode::Comparison {
                comparison,
                lhs,
                rhs,
            } => RuleExprView::Comparison {
                comparison: *comparison,
                lhs,
                rhs,
            },
            RuleNode::Select {
                condition,
                when_true,
                when_false,
            } => RuleExprView::Select {
                condition,
                when_true,
                when_false,
            },
            RuleNode::InterpolatedTable { table, input } => {
                RuleExprView::InterpolatedTable { table, input }
            }
        }
    }

    pub(crate) fn encode_payload_unframed(
        &self,
        writer: &mut CanonicalPayloadWriter,
    ) -> Result<(), CanonicalEncodingError> {
        writer.write_u16(1);
        writer.write_u16(1);
        encode_node(self, writer)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum WireVersion {
    V1,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum WireComparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}
impl From<ScalarComparison> for WireComparison {
    fn from(v: ScalarComparison) -> Self {
        match v {
            ScalarComparison::Equal => Self::Equal,
            ScalarComparison::NotEqual => Self::NotEqual,
            ScalarComparison::LessThan => Self::LessThan,
            ScalarComparison::LessThanOrEqual => Self::LessThanOrEqual,
            ScalarComparison::GreaterThan => Self::GreaterThan,
            ScalarComparison::GreaterThanOrEqual => Self::GreaterThanOrEqual,
        }
    }
}
#[derive(Serialize)]
struct RuleWireRef<'a> {
    rule_ir_version: WireVersion,
    numerical_semantics_version: WireVersion,
    expression: RuleNodeRef<'a>,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RuleNodeRef<'a> {
    Input {
        reference: &'a InputRef,
    },
    Parameter {
        reference: &'a ParameterRef,
    },
    Forcing {
        reference: &'a ForcingRef,
    },
    Projection {
        reference: &'a ProjectionRef,
    },
    Literal {
        value: f64,
    },
    Add {
        lhs: Box<RuleNodeRef<'a>>,
        rhs: Box<RuleNodeRef<'a>>,
    },
    Subtract {
        lhs: Box<RuleNodeRef<'a>>,
        rhs: Box<RuleNodeRef<'a>>,
    },
    Multiply {
        lhs: Box<RuleNodeRef<'a>>,
        rhs: Box<RuleNodeRef<'a>>,
    },
    Divide {
        lhs: Box<RuleNodeRef<'a>>,
        rhs: Box<RuleNodeRef<'a>>,
    },
    Minimum {
        lhs: Box<RuleNodeRef<'a>>,
        rhs: Box<RuleNodeRef<'a>>,
    },
    Maximum {
        lhs: Box<RuleNodeRef<'a>>,
        rhs: Box<RuleNodeRef<'a>>,
    },
    Clamp {
        value: Box<RuleNodeRef<'a>>,
        lower: Box<RuleNodeRef<'a>>,
        upper: Box<RuleNodeRef<'a>>,
    },
    Comparison {
        comparison: WireComparison,
        lhs: Box<RuleNodeRef<'a>>,
        rhs: Box<RuleNodeRef<'a>>,
    },
    Select {
        condition: Box<RuleNodeRef<'a>>,
        when_true: Box<RuleNodeRef<'a>>,
        when_false: Box<RuleNodeRef<'a>>,
    },
    InterpolatedTable {
        table: &'a InterpolatedTableRef,
        input: Box<RuleNodeRef<'a>>,
    },
}
fn node_ref(expr: &RuleExpr) -> RuleNodeRef<'_> {
    match &expr.node {
        RuleNode::Input(reference) => RuleNodeRef::Input { reference },
        RuleNode::Parameter(reference) => RuleNodeRef::Parameter { reference },
        RuleNode::Forcing(reference) => RuleNodeRef::Forcing { reference },
        RuleNode::Projection(reference) => RuleNodeRef::Projection { reference },
        RuleNode::Literal(value) => RuleNodeRef::Literal {
            value: value.value(),
        },
        RuleNode::Add(l, r) => RuleNodeRef::Add {
            lhs: Box::new(node_ref(l)),
            rhs: Box::new(node_ref(r)),
        },
        RuleNode::Subtract(l, r) => RuleNodeRef::Subtract {
            lhs: Box::new(node_ref(l)),
            rhs: Box::new(node_ref(r)),
        },
        RuleNode::Multiply(l, r) => RuleNodeRef::Multiply {
            lhs: Box::new(node_ref(l)),
            rhs: Box::new(node_ref(r)),
        },
        RuleNode::Divide(l, r) => RuleNodeRef::Divide {
            lhs: Box::new(node_ref(l)),
            rhs: Box::new(node_ref(r)),
        },
        RuleNode::Minimum(l, r) => RuleNodeRef::Minimum {
            lhs: Box::new(node_ref(l)),
            rhs: Box::new(node_ref(r)),
        },
        RuleNode::Maximum(l, r) => RuleNodeRef::Maximum {
            lhs: Box::new(node_ref(l)),
            rhs: Box::new(node_ref(r)),
        },
        RuleNode::Clamp {
            value,
            lower,
            upper,
        } => RuleNodeRef::Clamp {
            value: Box::new(node_ref(value)),
            lower: Box::new(node_ref(lower)),
            upper: Box::new(node_ref(upper)),
        },
        RuleNode::Comparison {
            comparison,
            lhs,
            rhs,
        } => RuleNodeRef::Comparison {
            comparison: (*comparison).into(),
            lhs: Box::new(node_ref(lhs)),
            rhs: Box::new(node_ref(rhs)),
        },
        RuleNode::Select {
            condition,
            when_true,
            when_false,
        } => RuleNodeRef::Select {
            condition: Box::new(node_ref(condition)),
            when_true: Box::new(node_ref(when_true)),
            when_false: Box::new(node_ref(when_false)),
        },
        RuleNode::InterpolatedTable { table, input } => RuleNodeRef::InterpolatedTable {
            table,
            input: Box::new(node_ref(input)),
        },
    }
}
impl Serialize for RuleExpr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        RuleWireRef {
            rule_ir_version: WireVersion::V1,
            numerical_semantics_version: WireVersion::V1,
            expression: node_ref(self),
        }
        .serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OwnedVersion {
    V1,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OwnedComparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}
impl From<OwnedComparison> for ScalarComparison {
    fn from(v: OwnedComparison) -> Self {
        match v {
            OwnedComparison::Equal => Self::Equal,
            OwnedComparison::NotEqual => Self::NotEqual,
            OwnedComparison::LessThan => Self::LessThan,
            OwnedComparison::LessThanOrEqual => Self::LessThanOrEqual,
            OwnedComparison::GreaterThan => Self::GreaterThan,
            OwnedComparison::GreaterThanOrEqual => Self::GreaterThanOrEqual,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleWire {
    rule_ir_version: OwnedVersion,
    numerical_semantics_version: OwnedVersion,
    expression: OwnedNode,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum OwnedNode {
    Input {
        reference: InputRef,
    },
    Parameter {
        reference: ParameterRef,
    },
    Forcing {
        reference: ForcingRef,
    },
    Projection {
        reference: ProjectionRef,
    },
    Literal {
        value: f64,
    },
    Add {
        lhs: Box<OwnedNode>,
        rhs: Box<OwnedNode>,
    },
    Subtract {
        lhs: Box<OwnedNode>,
        rhs: Box<OwnedNode>,
    },
    Multiply {
        lhs: Box<OwnedNode>,
        rhs: Box<OwnedNode>,
    },
    Divide {
        lhs: Box<OwnedNode>,
        rhs: Box<OwnedNode>,
    },
    Minimum {
        lhs: Box<OwnedNode>,
        rhs: Box<OwnedNode>,
    },
    Maximum {
        lhs: Box<OwnedNode>,
        rhs: Box<OwnedNode>,
    },
    Clamp {
        value: Box<OwnedNode>,
        lower: Box<OwnedNode>,
        upper: Box<OwnedNode>,
    },
    Comparison {
        comparison: OwnedComparison,
        lhs: Box<OwnedNode>,
        rhs: Box<OwnedNode>,
    },
    Select {
        condition: Box<OwnedNode>,
        when_true: Box<OwnedNode>,
        when_false: Box<OwnedNode>,
    },
    InterpolatedTable {
        table: InterpolatedTableRef,
        input: Box<OwnedNode>,
    },
}
fn build_owned(node: OwnedNode) -> Result<RuleExpr, RuleExprError> {
    let r = RuleIrVersion::V1;
    let s = NumericalSemanticsVersion::V1;
    match node {
        OwnedNode::Input { reference } => Ok(RuleExpr::input(r, s, reference)),
        OwnedNode::Parameter { reference } => Ok(RuleExpr::parameter(r, s, reference)),
        OwnedNode::Forcing { reference } => Ok(RuleExpr::forcing(r, s, reference)),
        OwnedNode::Projection { reference } => Ok(RuleExpr::projection(r, s, reference)),
        OwnedNode::Literal { value } => RuleExpr::literal(r, s, value),
        OwnedNode::Add { lhs, rhs } => RuleExpr::add(build_owned(*lhs)?, build_owned(*rhs)?),
        OwnedNode::Subtract { lhs, rhs } => {
            RuleExpr::subtract(build_owned(*lhs)?, build_owned(*rhs)?)
        }
        OwnedNode::Multiply { lhs, rhs } => {
            RuleExpr::multiply(build_owned(*lhs)?, build_owned(*rhs)?)
        }
        OwnedNode::Divide { lhs, rhs } => RuleExpr::divide(build_owned(*lhs)?, build_owned(*rhs)?),
        OwnedNode::Minimum { lhs, rhs } => {
            RuleExpr::minimum(build_owned(*lhs)?, build_owned(*rhs)?)
        }
        OwnedNode::Maximum { lhs, rhs } => {
            RuleExpr::maximum(build_owned(*lhs)?, build_owned(*rhs)?)
        }
        OwnedNode::Clamp {
            value,
            lower,
            upper,
        } => RuleExpr::clamp(
            build_owned(*value)?,
            build_owned(*lower)?,
            build_owned(*upper)?,
        ),
        OwnedNode::Comparison {
            comparison,
            lhs,
            rhs,
        } => RuleExpr::comparison(comparison.into(), build_owned(*lhs)?, build_owned(*rhs)?),
        OwnedNode::Select {
            condition,
            when_true,
            when_false,
        } => RuleExpr::select(
            build_owned(*condition)?,
            build_owned(*when_true)?,
            build_owned(*when_false)?,
        ),
        OwnedNode::InterpolatedTable { table, input } => {
            RuleExpr::interpolated_table(table, build_owned(*input)?)
        }
    }
}
impl<'de> Deserialize<'de> for RuleExpr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RuleWire::deserialize(deserializer)?;
        let _ = (wire.rule_ir_version, wire.numerical_semantics_version);
        build_owned(wire.expression).map_err(serde::de::Error::custom)
    }
}

fn encode_node(
    expr: &RuleExpr,
    w: &mut CanonicalPayloadWriter,
) -> Result<(), CanonicalEncodingError> {
    match &expr.node {
        RuleNode::Input(r) => {
            w.write_u8(0x00);
            w.write_u8(kind_tag(r.value_kind()));
            w.write_string(CanonicalField::RuleInputIdentity, r.id().as_str())?
        }
        RuleNode::Parameter(r) => {
            w.write_u8(0x01);
            w.write_u8(kind_tag(r.value_kind()));
            w.write_string(CanonicalField::RuleParameterIdentity, r.id().as_str())?
        }
        RuleNode::Forcing(r) => {
            w.write_u8(0x02);
            w.write_string(CanonicalField::RuleForcingIdentity, r.id().as_str())?
        }
        RuleNode::Projection(r) => {
            w.write_u8(0x03);
            w.write_u8(projection_kind_tag(r.value_kind()));
            w.write_string(CanonicalField::RuleProjectionIdentity, r.id().as_str())?
        }
        RuleNode::Literal(v) => {
            w.write_u8(0x04);
            w.write_scalar(CanonicalField::RuleLiteral, v.value())?
        }
        RuleNode::Add(l, r) => encode_binary(0x05, l, r, w)?,
        RuleNode::Subtract(l, r) => encode_binary(0x06, l, r, w)?,
        RuleNode::Multiply(l, r) => encode_binary(0x07, l, r, w)?,
        RuleNode::Divide(l, r) => encode_binary(0x08, l, r, w)?,
        RuleNode::Minimum(l, r) => encode_binary(0x09, l, r, w)?,
        RuleNode::Maximum(l, r) => encode_binary(0x0a, l, r, w)?,
        RuleNode::Clamp {
            value,
            lower,
            upper,
        } => {
            w.write_u8(0x0b);
            encode_node(value, w)?;
            encode_node(lower, w)?;
            encode_node(upper, w)?
        }
        RuleNode::Comparison {
            comparison,
            lhs,
            rhs,
        } => {
            w.write_u8(0x0c);
            w.write_u8(comparison_tag(*comparison));
            encode_node(lhs, w)?;
            encode_node(rhs, w)?
        }
        RuleNode::Select {
            condition,
            when_true,
            when_false,
        } => {
            w.write_u8(0x0d);
            encode_node(condition, w)?;
            encode_node(when_true, w)?;
            encode_node(when_false, w)?
        }
        RuleNode::InterpolatedTable { table, input } => {
            w.write_u8(0x0e);
            w.write_string(CanonicalField::RuleTableIdentity, table.id().as_str())?;
            encode_node(input, w)?
        }
    }
    Ok(())
}
fn encode_binary(
    tag: u8,
    l: &RuleExpr,
    r: &RuleExpr,
    w: &mut CanonicalPayloadWriter,
) -> Result<(), CanonicalEncodingError> {
    w.write_u8(tag);
    encode_node(l, w)?;
    encode_node(r, w)
}
fn kind_tag(k: ExpressionValueKind) -> u8 {
    match k {
        ExpressionValueKind::Scalar => 0,
        ExpressionValueKind::Truth => 1,
    }
}
fn projection_kind_tag(k: ProjectionValueKind) -> u8 {
    match k {
        ProjectionValueKind::Scalar => 0,
        ProjectionValueKind::Truth => 1,
        ProjectionValueKind::Extensive => 2,
    }
}
fn comparison_tag(c: ScalarComparison) -> u8 {
    match c {
        ScalarComparison::Equal => 0,
        ScalarComparison::NotEqual => 1,
        ScalarComparison::LessThan => 2,
        ScalarComparison::LessThanOrEqual => 3,
        ScalarComparison::GreaterThan => 4,
        ScalarComparison::GreaterThanOrEqual => 5,
    }
}
impl CanonicalEncode for RuleExpr {
    fn root_tag(&self) -> u16 {
        0x0016
    }
    fn encode_payload(&self, w: &mut CanonicalPayloadWriter) -> Result<(), CanonicalEncodingError> {
        w.write_u16(1);
        w.write_u16(1);
        encode_node(self, w)
    }
}
