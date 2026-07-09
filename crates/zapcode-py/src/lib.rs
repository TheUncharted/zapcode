use std::sync::Arc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString};

type PyObject = Py<PyAny>;

use zapcode_core::{
    ExecutionTrace, ResourceLimits, TraceSpan as CoreTraceSpan, TraceStatus, Value, VmState,
    ZapcodeError, ZapcodeSnapshot as CoreSnapshot,
};

// ---------------------------------------------------------------------------
// Value conversion: zapcode_core::Value <-> Python object
// ---------------------------------------------------------------------------

/// Convert a Python object to a `zapcode_core::Value`.
fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.cast::<PyBool>() {
        Ok(Value::Bool(b.is_true()))
    } else if let Ok(i) = obj.cast::<PyInt>() {
        let val: i64 = i.extract()?;
        Ok(Value::Int(val))
    } else if let Ok(f) = obj.cast::<PyFloat>() {
        let val: f64 = f.extract()?;
        Ok(Value::Float(val))
    } else if let Ok(s) = obj.cast::<PyString>() {
        let val: String = s.extract()?;
        Ok(Value::String(Arc::from(val.as_str())))
    } else if let Ok(list) = obj.cast::<PyList>() {
        let items: PyResult<Vec<Value>> = list.iter().map(|item| py_to_value(&item)).collect();
        Ok(Value::Array(items?))
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = indexmap::IndexMap::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            let val = py_to_value(&v)?;
            map.insert(Arc::from(key.as_str()), val);
        }
        Ok(Value::Object(map))
    } else {
        Err(PyRuntimeError::new_err(format!(
            "cannot convert Python type '{}' to Zapcode value",
            obj.get_type().name()?
        )))
    }
}

/// Convert a `zapcode_core::Value` to a Python object.
fn value_to_py(py: Python<'_>, val: &Value) -> PyResult<PyObject> {
    match val {
        Value::Undefined | Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Int(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
        Value::Float(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
        Value::String(s) => Ok(s.as_ref().into_pyobject(py)?.into_any().unbind()),
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.into_pyobject(py)?.into_any().unbind())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k.as_ref(), value_to_py(py, v)?)?;
            }
            Ok(dict.into_pyobject(py)?.into_any().unbind())
        }
        Value::Spread(inner) => value_to_py(py, inner),
        Value::Function(_) | Value::BuiltinMethod { .. } => {
            // Functions cannot be meaningfully represented in Python.
            Ok("<function>".into_pyobject(py)?.into_any().unbind())
        }
        Value::Generator(_) => Ok("<generator>".into_pyobject(py)?.into_any().unbind()),
    }
}

/// Convert a `ZapcodeError` to a Python `RuntimeError`.
fn zapcode_err(e: ZapcodeError) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

/// Extract input key-value pairs from an optional Python dict into `Vec<(String, Value)>`.
fn extract_inputs(inputs: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, Value)>> {
    match inputs {
        None => Ok(Vec::new()),
        Some(dict) => {
            let mut out = Vec::new();
            for (k, v) in dict.iter() {
                let key: String = k.extract()?;
                let val = py_to_value(&v)?;
                out.push((key, val));
            }
            Ok(out)
        }
    }
}

// ---------------------------------------------------------------------------
// Zapcode — main entry point
// ---------------------------------------------------------------------------

#[pyclass]
struct Zapcode {
    inner: zapcode_core::ZapcodeRun,
}

#[pymethods]
impl Zapcode {
    /// Create a new Zapcode instance.
    ///
    /// Args:
    ///     code: TypeScript source code to execute.
    ///     inputs: List of input variable names that will be injected at runtime.
    ///     external_functions: List of external function names the sandbox may call.
    ///     memory_limit_bytes: Maximum memory in bytes (default 32MB).
    ///     time_limit_ms: Maximum execution time in milliseconds (default 5000).
    ///     max_stack_depth: Maximum call stack depth (default 512).
    ///     max_allocations: Maximum number of heap allocations (default 100000).
    #[new]
    #[pyo3(signature = (code, inputs=None, external_functions=None, memory_limit_bytes=None, time_limit_ms=None, max_stack_depth=None, max_allocations=None))]
    fn new(
        code: String,
        inputs: Option<Vec<String>>,
        external_functions: Option<Vec<String>>,
        memory_limit_bytes: Option<usize>,
        time_limit_ms: Option<u64>,
        max_stack_depth: Option<usize>,
        max_allocations: Option<usize>,
    ) -> PyResult<Self> {
        let defaults = ResourceLimits::default();
        let limits = ResourceLimits {
            memory_limit_bytes: memory_limit_bytes.unwrap_or(defaults.memory_limit_bytes),
            time_limit_ms: time_limit_ms.unwrap_or(defaults.time_limit_ms),
            max_stack_depth: max_stack_depth.unwrap_or(defaults.max_stack_depth),
            max_allocations: max_allocations.unwrap_or(defaults.max_allocations),
        };
        let inner = zapcode_core::ZapcodeRun::new(
            code,
            inputs.unwrap_or_default(),
            external_functions.unwrap_or_default(),
            limits,
        )
        .map_err(zapcode_err)?;
        Ok(Self { inner })
    }

    /// Run the program to completion.
    ///
    /// Args:
    ///     inputs: Optional dict of input name -> value mappings.
    ///
    /// Returns:
    ///     A dict with keys "output" (the final value) and "stdout" (captured output).
    ///     If execution suspends on an external function, returns a dict with
    ///     "suspended", "function_name", "args", and "snapshot" keys instead.
    #[pyo3(signature = (inputs=None))]
    fn run(&self, py: Python<'_>, inputs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
        let input_values = extract_inputs(inputs)?;
        let result = self.inner.run(input_values).map_err(zapcode_err)?;
        run_result_to_py(py, result.state, &result.stdout, Some(&result.trace))
    }

    /// Start execution, returning raw state (for suspension / snapshot handling).
    ///
    /// Args:
    ///     inputs: Optional dict of input name -> value mappings.
    ///
    /// Returns:
    ///     Same shape as `run()`.
    #[pyo3(signature = (inputs=None))]
    fn start(&self, py: Python<'_>, inputs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
        let input_values = extract_inputs(inputs)?;
        let result = self.inner.run(input_values).map_err(zapcode_err)?;
        run_result_to_py(py, result.state, &result.stdout, Some(&result.trace))
    }
}

/// Convert a `TraceSpan` to a Python dict.
fn trace_span_to_py(py: Python<'_>, span: &CoreTraceSpan) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    dict.set_item("name", &span.name)?;
    dict.set_item("start_time_ms", span.start_time_ms)?;
    dict.set_item("end_time_ms", span.end_time_ms)?;
    dict.set_item("duration_us", span.duration_us)?;
    dict.set_item(
        "status",
        match span.status {
            TraceStatus::Ok => "ok",
            TraceStatus::Error => "error",
        },
    )?;
    let attrs = PyDict::new(py);
    for (k, v) in &span.attributes {
        attrs.set_item(k, v)?;
    }
    dict.set_item("attributes", attrs)?;
    let children = PyList::empty(py);
    for child in &span.children {
        children.append(trace_span_to_py(py, child)?)?;
    }
    dict.set_item("children", children)?;
    Ok(dict.into_pyobject(py)?.into_any().unbind())
}

/// Convert a `VmState` (+ optional stdout + trace) to a Python dict.
fn run_result_to_py(
    py: Python<'_>,
    state: VmState,
    stdout: &str,
    trace: Option<&ExecutionTrace>,
) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    match state {
        VmState::Complete(value) => {
            dict.set_item("output", value_to_py(py, &value)?)?;
            dict.set_item("stdout", stdout)?;
        }
        VmState::Suspended {
            function_name,
            args,
            snapshot,
        } => {
            dict.set_item("suspended", true)?;
            dict.set_item("function_name", &function_name)?;
            let py_args = PyList::empty(py);
            for arg in &args {
                py_args.append(value_to_py(py, arg)?)?;
            }
            dict.set_item("args", py_args)?;
            dict.set_item("snapshot", ZapcodeSnapshot { inner: snapshot })?;
            dict.set_item("stdout", stdout)?;
        }
    }
    if let Some(t) = trace {
        dict.set_item("trace", trace_span_to_py(py, &t.root)?)?;
    }
    Ok(dict.into_pyobject(py)?.into_any().unbind())
}

// ---------------------------------------------------------------------------
// ZapcodeSnapshot — snapshot / resume
// ---------------------------------------------------------------------------

#[pyclass]
struct ZapcodeSnapshot {
    inner: CoreSnapshot,
}

#[pymethods]
impl ZapcodeSnapshot {
    /// Serialize the snapshot to bytes.
    fn dump(&self) -> PyResult<Vec<u8>> {
        self.inner.dump().map_err(zapcode_err)
    }

    /// Deserialize a snapshot from bytes.
    #[staticmethod]
    fn load(bytes: Vec<u8>) -> PyResult<Self> {
        let inner = CoreSnapshot::load(&bytes).map_err(zapcode_err)?;
        Ok(Self { inner })
    }

    /// Resume execution with a return value from the external function.
    ///
    /// Args:
    ///     return_value: The value to return to the suspended external call.
    ///
    /// Returns:
    ///     A dict with either "output" or "suspended" keys (same shape as Zapcode.run()).
    fn resume(&self, py: Python<'_>, return_value: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let val = py_to_value(return_value)?;
        let state = self.inner.clone().resume(val).map_err(zapcode_err)?;
        run_result_to_py(py, state, "", None)
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

#[pymodule]
fn zapcode(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Zapcode>()?;
    m.add_class::<ZapcodeSnapshot>()?;
    Ok(())
}
