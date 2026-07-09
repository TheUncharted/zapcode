use std::sync::Arc;

use js_sys::{Array, Object, Reflect};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use zapcode_core::{
    ExecutionTrace, ResourceLimits, TraceSpan as CoreTraceSpan, TraceStatus, Value, VmState,
    ZapcodeError, ZapcodeSnapshot as CoreSnapshot,
};

// ---------------------------------------------------------------------------
// Value conversion: zapcode_core::Value <-> JsValue
// ---------------------------------------------------------------------------

/// Convert a `JsValue` to a `zapcode_core::Value`.
fn js_to_value(js: &JsValue) -> Result<Value, JsError> {
    if js.is_undefined() {
        Ok(Value::Undefined)
    } else if js.is_null() {
        Ok(Value::Null)
    } else if let Some(b) = js.as_bool() {
        Ok(Value::Bool(b))
    } else if let Some(n) = js.as_f64() {
        // Represent whole numbers as Int for fidelity with the core VM.
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            Ok(Value::Int(n as i64))
        } else {
            Ok(Value::Float(n))
        }
    } else if let Some(s) = js.as_string() {
        Ok(Value::String(Arc::from(s.as_str())))
    } else if Array::is_array(js) {
        let arr = Array::from(js);
        let mut items = Vec::with_capacity(arr.length() as usize);
        for i in 0..arr.length() {
            items.push(js_to_value(&arr.get(i))?);
        }
        Ok(Value::Array(items))
    } else if js.is_object() {
        let obj = Object::from(js.clone());
        let entries = Object::entries(&obj);
        let mut map = indexmap::IndexMap::new();
        for i in 0..entries.length() {
            let pair = Array::from(&entries.get(i));
            let key = pair
                .get(0)
                .as_string()
                .ok_or_else(|| JsError::new("object keys must be strings"))?;
            let val = js_to_value(&pair.get(1))?;
            map.insert(Arc::from(key.as_str()), val);
        }
        Ok(Value::Object(map))
    } else {
        Err(JsError::new(&format!(
            "cannot convert JS value to Zapcode value: {:?}",
            js
        )))
    }
}

/// Convert a `zapcode_core::Value` to a `JsValue`.
fn value_to_js(val: &Value) -> Result<JsValue, JsError> {
    match val {
        Value::Undefined => Ok(JsValue::undefined()),
        Value::Null => Ok(JsValue::null()),
        Value::Bool(b) => Ok(JsValue::from(*b)),
        Value::Int(n) => Ok(JsValue::from(*n as f64)),
        Value::Float(n) => Ok(JsValue::from(*n)),
        Value::String(s) => Ok(JsValue::from_str(s.as_ref())),
        Value::Array(arr) => {
            let js_arr = Array::new_with_length(arr.len() as u32);
            for (i, item) in arr.iter().enumerate() {
                js_arr.set(i as u32, value_to_js(item)?);
            }
            Ok(js_arr.into())
        }
        Value::Object(map) => {
            let obj = Object::new();
            for (k, v) in map {
                Reflect::set(&obj, &JsValue::from_str(k.as_ref()), &value_to_js(v)?)
                    .map_err(|_| JsError::new("failed to set object property"))?;
            }
            Ok(obj.into())
        }
        Value::Spread(inner) => value_to_js(inner),
        Value::Function(_) | Value::BuiltinMethod { .. } => Ok(JsValue::from_str("<function>")),
        Value::Generator(_) => Ok(JsValue::from_str("<generator>")),
    }
}

/// Convert a `ZapcodeError` to a `JsError`.
fn zapcode_err(e: ZapcodeError) -> JsError {
    JsError::new(&e.to_string())
}

// ---------------------------------------------------------------------------
// Options structs (deserialized from JsValue via serde-wasm-bindgen)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ZapcodeOptions {
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    external_functions: Vec<String>,
    #[serde(default)]
    memory_limit_bytes: Option<usize>,
    #[serde(default)]
    time_limit_ms: Option<u64>,
    #[serde(default)]
    max_stack_depth: Option<usize>,
    #[serde(default)]
    max_allocations: Option<usize>,
}

// ---------------------------------------------------------------------------
// Zapcode — main entry point
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct Zapcode {
    inner: zapcode_core::ZapcodeRun,
}

#[wasm_bindgen]
impl Zapcode {
    /// Create a new Zapcode instance.
    ///
    /// @param code - TypeScript source code to execute.
    /// @param options - Optional configuration object with fields:
    ///   - inputs: string[] - Variable names injected at runtime.
    ///   - externalFunctions: string[] - Function names the sandbox may call.
    ///   - memoryLimitBytes: number - Maximum memory in bytes.
    ///   - timeLimitMs: number - Maximum execution time in milliseconds.
    ///   - maxStackDepth: number - Maximum call stack depth.
    ///   - maxAllocations: number - Maximum heap allocations.
    #[wasm_bindgen(constructor)]
    pub fn new(code: &str, options: JsValue) -> Result<Zapcode, JsError> {
        let opts: ZapcodeOptions = if options.is_undefined() || options.is_null() {
            ZapcodeOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options)
                .map_err(|e| JsError::new(&format!("invalid options: {}", e)))?
        };

        let defaults = ResourceLimits::default();
        let limits = ResourceLimits {
            memory_limit_bytes: opts
                .memory_limit_bytes
                .unwrap_or(defaults.memory_limit_bytes),
            time_limit_ms: opts.time_limit_ms.unwrap_or(defaults.time_limit_ms),
            max_stack_depth: opts.max_stack_depth.unwrap_or(defaults.max_stack_depth),
            max_allocations: opts.max_allocations.unwrap_or(defaults.max_allocations),
        };

        let inner = zapcode_core::ZapcodeRun::new(
            code.to_string(),
            opts.inputs,
            opts.external_functions,
            limits,
        )
        .map_err(zapcode_err)?;

        Ok(Self { inner })
    }

    /// Run the program to completion.
    ///
    /// @param inputs - Optional object mapping input names to values.
    /// @returns An object with `output` and `stdout` keys on completion,
    ///          or `suspended`, `functionName`, `args`, and `snapshot` keys on suspension.
    pub fn run(&self, inputs: JsValue) -> Result<JsValue, JsError> {
        let input_values = extract_inputs(&inputs)?;
        let result = self.inner.run(input_values).map_err(zapcode_err)?;
        vm_state_to_js(result.state, &result.stdout, Some(&result.trace))
    }

    /// Start execution, returning raw state (for suspension / snapshot handling).
    ///
    /// @param inputs - Optional object mapping input names to values.
    /// @returns Same shape as `run()`.
    pub fn start(&self, inputs: JsValue) -> Result<JsValue, JsError> {
        let input_values = extract_inputs(&inputs)?;
        let result = self.inner.run(input_values).map_err(zapcode_err)?;
        vm_state_to_js(result.state, &result.stdout, Some(&result.trace))
    }
}

/// Extract input key-value pairs from a JsValue (expected to be an object or undefined/null).
fn extract_inputs(inputs: &JsValue) -> Result<Vec<(String, Value)>, JsError> {
    if inputs.is_undefined() || inputs.is_null() {
        return Ok(Vec::new());
    }
    let obj = Object::from(inputs.clone());
    let entries = Object::entries(&obj);
    let mut out = Vec::with_capacity(entries.length() as usize);
    for i in 0..entries.length() {
        let pair = Array::from(&entries.get(i));
        let key = pair
            .get(0)
            .as_string()
            .ok_or_else(|| JsError::new("input keys must be strings"))?;
        let val = js_to_value(&pair.get(1))?;
        out.push((key, val));
    }
    Ok(out)
}

/// Convert a `TraceSpan` to a JS object.
fn trace_span_to_js(span: &CoreTraceSpan) -> Result<JsValue, JsError> {
    let obj = Object::new();
    Reflect::set(&obj, &"name".into(), &JsValue::from_str(&span.name))
        .map_err(|_| JsError::new("failed to set trace field"))?;
    Reflect::set(
        &obj,
        &"startTimeMs".into(),
        &JsValue::from(span.start_time_ms as f64),
    )
    .map_err(|_| JsError::new("failed to set trace field"))?;
    Reflect::set(
        &obj,
        &"endTimeMs".into(),
        &JsValue::from(span.end_time_ms as f64),
    )
    .map_err(|_| JsError::new("failed to set trace field"))?;
    Reflect::set(
        &obj,
        &"durationUs".into(),
        &JsValue::from(span.duration_us as f64),
    )
    .map_err(|_| JsError::new("failed to set trace field"))?;
    Reflect::set(
        &obj,
        &"status".into(),
        &JsValue::from_str(match span.status {
            TraceStatus::Ok => "ok",
            TraceStatus::Error => "error",
        }),
    )
    .map_err(|_| JsError::new("failed to set trace field"))?;

    let attrs = Object::new();
    for (k, v) in &span.attributes {
        Reflect::set(&attrs, &JsValue::from_str(k), &JsValue::from_str(v))
            .map_err(|_| JsError::new("failed to set trace attribute"))?;
    }
    Reflect::set(&obj, &"attributes".into(), &attrs.into())
        .map_err(|_| JsError::new("failed to set trace field"))?;

    let children = Array::new_with_length(span.children.len() as u32);
    for (i, child) in span.children.iter().enumerate() {
        children.set(i as u32, trace_span_to_js(child)?);
    }
    Reflect::set(&obj, &"children".into(), &children.into())
        .map_err(|_| JsError::new("failed to set trace field"))?;

    Ok(obj.into())
}

/// Convert a `VmState` (+ optional stdout + trace) to a JS object.
fn vm_state_to_js(
    state: VmState,
    stdout: &str,
    trace: Option<&ExecutionTrace>,
) -> Result<JsValue, JsError> {
    let obj = Object::new();
    match state {
        VmState::Complete(value) => {
            Reflect::set(&obj, &JsValue::from_str("output"), &value_to_js(&value)?)
                .map_err(|_| JsError::new("failed to set output"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("stdout"),
                &JsValue::from_str(stdout),
            )
            .map_err(|_| JsError::new("failed to set stdout"))?;
        }
        VmState::Suspended {
            function_name,
            args,
            snapshot,
        } => {
            Reflect::set(&obj, &JsValue::from_str("suspended"), &JsValue::from(true))
                .map_err(|_| JsError::new("failed to set suspended"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("functionName"),
                &JsValue::from_str(&function_name),
            )
            .map_err(|_| JsError::new("failed to set functionName"))?;
            let js_args = Array::new_with_length(args.len() as u32);
            for (i, arg) in args.iter().enumerate() {
                js_args.set(i as u32, value_to_js(arg)?);
            }
            Reflect::set(&obj, &JsValue::from_str("args"), &js_args.into())
                .map_err(|_| JsError::new("failed to set args"))?;
            let snap = ZapcodeSnapshot { inner: snapshot };
            Reflect::set(&obj, &JsValue::from_str("snapshot"), &snap.into_js()?)
                .map_err(|_| JsError::new("failed to set snapshot"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("stdout"),
                &JsValue::from_str(stdout),
            )
            .map_err(|_| JsError::new("failed to set stdout"))?;
        }
    }
    if let Some(t) = trace {
        Reflect::set(&obj, &"trace".into(), &trace_span_to_js(&t.root)?)
            .map_err(|_| JsError::new("failed to set trace"))?;
    }
    Ok(obj.into())
}

// ---------------------------------------------------------------------------
// ZapcodeSnapshot — snapshot / resume
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct ZapcodeSnapshot {
    inner: CoreSnapshot,
}

impl ZapcodeSnapshot {
    /// Convert snapshot to a JsValue (for embedding in result objects).
    fn into_js(self) -> Result<JsValue, JsError> {
        // Return as a wasm_bindgen class instance.
        Ok(JsValue::from(self))
    }
}

#[wasm_bindgen]
impl ZapcodeSnapshot {
    /// Serialize the snapshot to bytes (Uint8Array).
    pub fn dump(&self) -> Result<Vec<u8>, JsError> {
        self.inner.dump().map_err(zapcode_err)
    }

    /// Deserialize a snapshot from bytes.
    pub fn load(bytes: &[u8]) -> Result<ZapcodeSnapshot, JsError> {
        let inner = CoreSnapshot::load(bytes).map_err(zapcode_err)?;
        Ok(Self { inner })
    }

    /// Resume execution with a return value from the external function.
    ///
    /// @param return_value - The value to return to the suspended external call.
    /// @returns Same shape as `Zapcode.run()`.
    pub fn resume(&self, return_value: JsValue) -> Result<JsValue, JsError> {
        let val = js_to_value(&return_value)?;
        let state = self.inner.clone().resume(val).map_err(zapcode_err)?;
        vm_state_to_js(state, "", None)
    }
}
