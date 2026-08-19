//! python_binding : PythonModelData ⇀ ModelArtifactHandle
//!
//! This crate is only a containment and transport boundary. Domain validation remains in
//! `incidence-core`'s public model-document decoder and artifact constructor.

use incidence_core::model_artifact::ModelArtifact;
use incidence_core::model_document::ModelDocument;
use incidence_core::numerical_semantics::{NumericalSemanticsVersion, ScalarComparison};
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef,
    ParameterId, ParameterRef, ProjectionId, ProjectionRef, ProjectionValueKind, TableId,
};
use incidence_core::versions::RuleIrVersion;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::any::Any;
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

const MAX_DOCUMENT_NESTING: usize = 256;

/// An immutable model validated and owned by the Rust core.
#[pyclass(frozen, module = "incidence._incidence")]
struct CompiledModel {
    _artifact: ModelArtifact,
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

fn contain<T>(operation: impl FnOnce() -> PyResult<T>) -> PyResult<T> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(payload) => Err(PyRuntimeError::new_err(format!(
            "incidence boundary contained panic: {}",
            panic_message(payload)
        ))),
    }
}

enum TraversalFrame<'py> {
    Enter {
        value: Bound<'py, PyAny>,
        depth: usize,
    },
    Exit(usize),
}

fn check_document_nesting(document: &Bound<'_, PyAny>) -> PyResult<()> {
    let mut frames = vec![TraversalFrame::Enter {
        value: document.clone(),
        depth: 0,
    }];
    let mut active_containers = HashSet::new();

    while let Some(frame) = frames.pop() {
        let TraversalFrame::Enter { value, depth } = frame else {
            if let TraversalFrame::Exit(identity) = frame {
                active_containers.remove(&identity);
            }
            continue;
        };
        if depth > MAX_DOCUMENT_NESTING {
            return Err(PyValueError::new_err(format!(
                "invalid model document encoding: document nesting depth exceeds maximum {MAX_DOCUMENT_NESTING}"
            )));
        }

        let children = if let Ok(mapping) = value.cast::<PyDict>() {
            mapping.iter().map(|(_, child)| child).collect::<Vec<_>>()
        } else if let Ok(sequence) = value.cast::<PyList>() {
            sequence.iter().collect::<Vec<_>>()
        } else if let Ok(sequence) = value.cast::<PyTuple>() {
            sequence.iter().collect::<Vec<_>>()
        } else {
            continue;
        };

        let identity = value.as_ptr() as usize;
        if !active_containers.insert(identity) {
            return Err(PyValueError::new_err(
                "invalid model document encoding: cyclic Python container",
            ));
        }
        frames.push(TraversalFrame::Exit(identity));
        frames.extend(children.into_iter().map(|child| TraversalFrame::Enter {
            value: child,
            depth: depth + 1,
        }));
    }
    Ok(())
}

fn compile_document(document: &Bound<'_, PyAny>) -> PyResult<CompiledModel> {
    check_document_nesting(document)?;
    let decoded: ModelDocument = pythonize::depythonize(document).map_err(|error| {
        PyValueError::new_err(format!("invalid model document encoding: {error}"))
    })?;
    let artifact = decoded
        .into_artifact()
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(CompiledModel {
        _artifact: artifact,
    })
}

/// Decode and validate a complete plain-data model document.
///
/// Every panic raised while crossing into the Rust decoder or core is converted to an ordinary
/// Python `RuntimeError`. It can never become PyO3's `PanicException`.
#[pyfunction]
fn compile_model(document: &Bound<'_, PyAny>) -> PyResult<CompiledModel> {
    contain(|| compile_document(document))
}

fn expression(document: &Bound<'_, PyAny>) -> PyResult<RuleExpr> {
    check_document_nesting(document)?;
    pythonize::depythonize(document)
        .map_err(|error| PyValueError::new_err(format!("invalid rule expression: {error}")))
}

fn authored_expression<'py>(py: Python<'py>, expression: &RuleExpr) -> PyResult<Bound<'py, PyAny>> {
    pythonize::pythonize(py, expression)
        .map_err(|error| PyValueError::new_err(format!("cannot encode rule expression: {error}")))
}

fn scalar_kind(value_kind: &str) -> PyResult<ExpressionValueKind> {
    match value_kind {
        "scalar" => Ok(ExpressionValueKind::Scalar),
        "truth" => Ok(ExpressionValueKind::Truth),
        other => Err(PyValueError::new_err(format!(
            "expression value kind must be `scalar` or `truth`, got `{other}`"
        ))),
    }
}

fn projection_kind(value_kind: &str) -> PyResult<ProjectionValueKind> {
    match value_kind {
        "extensive" => Ok(ProjectionValueKind::Extensive),
        "scalar" => Ok(ProjectionValueKind::Scalar),
        "truth" => Ok(ProjectionValueKind::Truth),
        other => Err(PyValueError::new_err(format!(
            "projection value kind must be `extensive`, `scalar`, or `truth`, got `{other}`"
        ))),
    }
}

fn rule_error(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// Author a finite scalar literal as a serialisable rule-expression document.
#[pyfunction]
fn literal<'py>(py: Python<'py>, value: f64) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let value = RuleExpr::literal(RuleIrVersion::V1, NumericalSemanticsVersion::V1, value)
            .map_err(rule_error)?;
        authored_expression(py, &value)
    })
}

/// Author a declared rule-parameter reference.
#[pyfunction(signature = (id, value_kind = "scalar"))]
fn param<'py>(py: Python<'py>, id: &str, value_kind: &str) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let id = ParameterId::parse(id).map_err(rule_error)?;
        let value = RuleExpr::parameter(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            ParameterRef::new(id, scalar_kind(value_kind)?),
        );
        authored_expression(py, &value)
    })
}

/// Author a generic bound-input reference.
#[pyfunction(signature = (id, value_kind = "scalar"))]
fn input<'py>(py: Python<'py>, id: &str, value_kind: &str) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let id = InputId::parse(id).map_err(rule_error)?;
        let value = RuleExpr::input(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            InputRef::new(id, scalar_kind(value_kind)?),
        );
        authored_expression(py, &value)
    })
}

/// Author a forcing-series reference.
#[pyfunction]
fn forcing<'py>(py: Python<'py>, id: &str) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let id = ForcingId::parse(id).map_err(rule_error)?;
        let value = RuleExpr::forcing(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            ForcingRef::new(id),
        );
        authored_expression(py, &value)
    })
}

/// Author a typed projection reference.
#[pyfunction(signature = (id, value_kind = "extensive"))]
fn projection<'py>(py: Python<'py>, id: &str, value_kind: &str) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let id = ProjectionId::parse(id).map_err(rule_error)?;
        let value = RuleExpr::projection(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            ProjectionRef::new(id, projection_kind(value_kind)?),
        );
        authored_expression(py, &value)
    })
}

fn binary<'py>(
    py: Python<'py>,
    lhs: &Bound<'_, PyAny>,
    rhs: &Bound<'_, PyAny>,
    constructor: fn(
        RuleExpr,
        RuleExpr,
    ) -> Result<RuleExpr, incidence_core::rule_expression::RuleExprError>,
) -> PyResult<Bound<'py, PyAny>> {
    let value = constructor(expression(lhs)?, expression(rhs)?).map_err(rule_error)?;
    authored_expression(py, &value)
}

/// Author an ordered addition.
#[pyfunction]
fn add<'py>(
    py: Python<'py>,
    lhs: &Bound<'_, PyAny>,
    rhs: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| binary(py, lhs, rhs, RuleExpr::add))
}

/// Author an ordered multiplication.
#[pyfunction]
fn mul<'py>(
    py: Python<'py>,
    lhs: &Bound<'_, PyAny>,
    rhs: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| binary(py, lhs, rhs, RuleExpr::multiply))
}

/// Author an ordered minimum.
#[pyfunction(name = "min")]
fn minimum<'py>(
    py: Python<'py>,
    lhs: &Bound<'_, PyAny>,
    rhs: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| binary(py, lhs, rhs, RuleExpr::minimum))
}

/// Author an ordered maximum.
#[pyfunction(name = "max")]
fn maximum<'py>(
    py: Python<'py>,
    lhs: &Bound<'_, PyAny>,
    rhs: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| binary(py, lhs, rhs, RuleExpr::maximum))
}

/// Author a scalar clamp.
#[pyfunction]
fn clamp<'py>(
    py: Python<'py>,
    value: &Bound<'_, PyAny>,
    lower: &Bound<'_, PyAny>,
    upper: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let value = RuleExpr::clamp(expression(value)?, expression(lower)?, expression(upper)?)
            .map_err(rule_error)?;
        authored_expression(py, &value)
    })
}

/// Author a scalar comparison used by `select` conditions.
#[pyfunction]
fn compare<'py>(
    py: Python<'py>,
    comparison: &str,
    lhs: &Bound<'_, PyAny>,
    rhs: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let comparison = match comparison {
            "equal" => ScalarComparison::Equal,
            "not_equal" => ScalarComparison::NotEqual,
            "less_than" => ScalarComparison::LessThan,
            "less_than_or_equal" => ScalarComparison::LessThanOrEqual,
            "greater_than" => ScalarComparison::GreaterThan,
            "greater_than_or_equal" => ScalarComparison::GreaterThanOrEqual,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown scalar comparison `{other}`"
                )));
            }
        };
        let value = RuleExpr::comparison(comparison, expression(lhs)?, expression(rhs)?)
            .map_err(rule_error)?;
        authored_expression(py, &value)
    })
}

/// Author a truth-directed scalar selection.
#[pyfunction]
fn select<'py>(
    py: Python<'py>,
    condition: &Bound<'_, PyAny>,
    when_true: &Bound<'_, PyAny>,
    when_false: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let value = RuleExpr::select(
            expression(condition)?,
            expression(when_true)?,
            expression(when_false)?,
        )
        .map_err(rule_error)?;
        authored_expression(py, &value)
    })
}

/// Author an interpolation-table lookup.
#[pyfunction]
fn table_lookup<'py>(
    py: Python<'py>,
    table: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| {
        let table = TableId::parse(table).map_err(rule_error)?;
        let value =
            RuleExpr::interpolated_table(InterpolatedTableRef::new(table), expression(value)?)
                .map_err(rule_error)?;
        authored_expression(py, &value)
    })
}

/// Decode and re-encode an expression through the authoritative Rust IR.
#[pyfunction]
fn roundtrip_expression<'py>(
    py: Python<'py>,
    value: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    contain(|| authored_expression(py, &expression(value)?))
}

fn initialize(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<CompiledModel>()?;
    module.add_function(wrap_pyfunction!(compile_model, module)?)?;
    module.add_function(wrap_pyfunction!(literal, module)?)?;
    module.add_function(wrap_pyfunction!(param, module)?)?;
    module.add_function(wrap_pyfunction!(input, module)?)?;
    module.add_function(wrap_pyfunction!(forcing, module)?)?;
    module.add_function(wrap_pyfunction!(projection, module)?)?;
    module.add_function(wrap_pyfunction!(add, module)?)?;
    module.add_function(wrap_pyfunction!(mul, module)?)?;
    module.add_function(wrap_pyfunction!(minimum, module)?)?;
    module.add_function(wrap_pyfunction!(maximum, module)?)?;
    module.add_function(wrap_pyfunction!(clamp, module)?)?;
    module.add_function(wrap_pyfunction!(compare, module)?)?;
    module.add_function(wrap_pyfunction!(select, module)?)?;
    module.add_function(wrap_pyfunction!(table_lookup, module)?)?;
    module.add_function(wrap_pyfunction!(roundtrip_expression, module)?)?;
    Ok(())
}

/// Python module initialiser.
#[pymodule]
fn _incidence(module: &Bound<'_, PyModule>) -> PyResult<()> {
    contain(|| initialize(module))
}
