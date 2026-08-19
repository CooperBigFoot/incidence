//! python_binding : PythonModelData ⇀ ModelArtifactHandle
//!
//! This crate is only a containment and transport boundary. Domain validation remains in
//! `incidence-core`'s public model-document decoder and artifact constructor.

use incidence_core::model_artifact::ModelArtifact;
use incidence_core::model_document::ModelDocument;
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

fn initialize(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<CompiledModel>()?;
    module.add_function(wrap_pyfunction!(compile_model, module)?)?;
    Ok(())
}

/// Python module initialiser.
#[pymodule]
fn _incidence(module: &Bound<'_, PyModule>) -> PyResult<()> {
    contain(|| initialize(module))
}
