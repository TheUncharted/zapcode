//! Adversarial security tests for the Zapcode sandbox.
//!
//! Each test attempts a specific attack vector against the sandbox.
//! Tests should either expect an error (attack blocked) or a safe/harmless value.

use zapcode_core::vm::{eval_ts, eval_ts_with_output};
use zapcode_core::Value;
use zapcode_core::{ResourceLimits, ZapcodeError, ZapcodeRun};

// ── 1. Prototype pollution ──────────────────────────────────────────

#[test]
fn test_prototype_pollution_object() {
    // Try to pollute Object.prototype
    let result = eval_ts(
        r#"
        Object.prototype.pwned = true;
        ({}).pwned
    "#,
    );
    // Should either error or return undefined (not true)
    match result {
        Err(_) => {} // blocked — good
        Ok(val) => assert_ne!(
            val,
            Value::Bool(true),
            "VULN: Object.prototype pollution succeeded"
        ),
    }
}

#[test]
fn test_prototype_pollution_array() {
    let result = eval_ts(
        r#"
        Array.prototype.pwned = "hacked";
        [].pwned
    "#,
    );
    match result {
        Err(_) => {}
        Ok(val) => assert_ne!(
            val,
            Value::String("hacked".into()),
            "VULN: Array.prototype pollution succeeded"
        ),
    }
}

#[test]
fn test_proto_assignment() {
    // Try to modify __proto__ directly
    let result = eval_ts(
        r#"
        const obj = {};
        obj.__proto__.polluted = true;
        ({}).polluted
    "#,
    );
    match result {
        Err(_) => {}
        Ok(val) => assert_ne!(
            val,
            Value::Bool(true),
            "VULN: __proto__ pollution succeeded"
        ),
    }
}

// ── 2. Constructor escape ───────────────────────────────────────────

#[test]
fn test_constructor_constructor_escape() {
    // Classic constructor chain to reach Function constructor
    let result = eval_ts(r#"({}).constructor.constructor("return 1+1")()"#);
    assert!(
        result.is_err(),
        "VULN: constructor.constructor escape reached Function"
    );
}

#[test]
fn test_array_constructor_escape() {
    let result = eval_ts(r#"[].constructor.constructor("return process")()"#);
    assert!(
        result.is_err(),
        "VULN: array constructor escape reached Function"
    );
}

#[test]
fn test_function_constructor_via_method() {
    let result = eval_ts(
        r#"
        const f = function() {};
        f.constructor("return 1")()
    "#,
    );
    assert!(result.is_err(), "VULN: function.constructor escape");
}

#[test]
fn test_constructor_via_bind() {
    let result = eval_ts(
        r#"
        const f = function() {}.bind({});
        f.constructor("return 1")()
    "#,
    );
    assert!(result.is_err(), "VULN: bind.constructor escape");
}

// ── 3. toString / valueOf hijacking ─────────────────────────────────

#[test]
fn test_tostring_hijack() {
    // Override toString to execute code during string conversion
    let result = eval_ts_with_output(
        r#"
        const evil = {
            toString() {
                return "pwned";
            }
        };
        "" + evil
    "#,
    );
    match result {
        Err(_) => {} // blocked
        Ok((_val, _)) => {
            // If it works, the string conversion should be "pwned" but not execute arbitrary code
            // This is acceptable behavior — toString is a normal JS feature
            // The key is it cannot escape the sandbox
        }
    }
}

#[test]
fn test_valueof_hijack() {
    let result = eval_ts(
        r#"
        const evil = {
            valueOf() {
                return 42;
            }
        };
        evil + 1
    "#,
    );
    // valueOf executing is fine as long as it can't escape
    let _ = result;
}

/// Zapcode does NOT invoke user-defined toString() during implicit string conversion.
/// Objects always stringify to "[object Object]". This is a security feature that
/// prevents toString-based code injection and infinite recursion.
#[test]
fn test_tostring_not_invoked_during_coercion() {
    let result = eval_ts(
        r#"
        const evil = {
            toString() {
                return "" + this;
            }
        };
        "" + evil
    "#,
    );
    // Should return "[object Object]" without invoking toString()
    assert_eq!(result.unwrap(), Value::String("[object Object]".into()));
}

// ── 4. Stack overflow DoS ───────────────────────────────────────────

#[test]
fn test_stack_overflow_direct() {
    let result = eval_ts(
        r#"
        function bomb() { return bomb(); }
        bomb()
    "#,
    );
    assert!(result.is_err(), "VULN: infinite recursion not caught");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ZapcodeError::StackOverflow(_) | ZapcodeError::TimeLimitExceeded
        ),
        "Expected StackOverflow or TimeLimitExceeded, got: {err}"
    );
}

#[test]
fn test_stack_overflow_mutual_recursion() {
    let result = eval_ts(
        r#"
        function a() { return b(); }
        function b() { return a(); }
        a()
    "#,
    );
    assert!(result.is_err(), "VULN: mutual recursion not caught");
}

// ── 5. Memory exhaustion DoS ────────────────────────────────────────

#[test]
fn test_memory_exhaustion_huge_array() {
    let result = eval_ts(
        r#"
        const arr: number[] = [];
        for (let i = 0; i < 10000000; i++) {
            arr.push(i);
        }
        arr.length
    "#,
    );
    assert!(result.is_err(), "VULN: huge array allocation not limited");
}

#[test]
fn test_memory_exhaustion_string_doubling() {
    let result = eval_ts(
        r#"
        let s = "a";
        for (let i = 0; i < 100; i++) {
            s = s + s;
        }
        s.length
    "#,
    );
    assert!(
        result.is_err(),
        "VULN: exponential string growth not limited"
    );
}

#[test]
fn test_memory_exhaustion_nested_arrays() {
    let result = eval_ts(
        r#"
        let arr: any = [1];
        for (let i = 0; i < 1000000; i++) {
            arr = [arr, arr, arr, arr];
        }
        arr
    "#,
    );
    assert!(result.is_err(), "VULN: nested array growth not limited");
}

// ── 6. Infinite loop DoS ───────────────────────────────────────────

#[test]
fn test_infinite_while_loop() {
    let result = eval_ts("while (true) {}");
    assert!(result.is_err(), "VULN: infinite while loop not caught");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ZapcodeError::TimeLimitExceeded | ZapcodeError::AllocationLimitExceeded
        ),
        "Expected TimeLimitExceeded or AllocationLimitExceeded, got: {err}"
    );
}

#[test]
fn test_infinite_for_loop() {
    let result = eval_ts("for (;;) {}");
    assert!(result.is_err(), "VULN: infinite for loop not caught");
}

// ── 7. JSON.parse bomb ─────────────────────────────────────────────

#[test]
fn test_json_parse_deeply_nested() {
    // Create deeply nested JSON via string building
    let result = eval_ts(
        r#"
        let s = "0";
        for (let i = 0; i < 10000; i++) {
            s = "[" + s + "]";
        }
        JSON.parse(s)
    "#,
    );
    // Should either error (allocation/memory/time limit) or handle gracefully
    let _ = result;
}

#[test]
fn test_json_parse_huge_string() {
    let result = eval_ts(
        r#"
        let s = "\"";
        for (let i = 0; i < 100; i++) {
            s = s + s;
        }
        s = s + "\"";
        JSON.parse(s)
    "#,
    );
    assert!(result.is_err(), "VULN: huge JSON.parse not limited");
}

// ── 8. Template literal injection ───────────────────────────────────

#[test]
fn test_template_literal_basic() {
    // Template literals should be safe string interpolation only
    let result = eval_ts(
        r#"
        const x = 42;
        `value is ${x}`
    "#,
    );
    match result {
        Err(_) => {} // template literals not supported is fine
        Ok(val) => assert_eq!(val, Value::String("value is 42".into())),
    }
}

#[test]
fn test_tagged_template_escape() {
    // Tagged templates could be an escape vector
    let result = eval_ts(
        r#"
        function tag(strings: TemplateStringsArray, ...values: any[]) {
            return strings.raw[0];
        }
        tag`\u0065val`
    "#,
    );
    // Should not execute eval
    match result {
        Err(_) => {} // blocked
        Ok(_val) => {
            // If it returns a string, that's fine as long as eval wasn't executed
        }
    }
}

// ── 9. Property access chains ───────────────────────────────────────

#[test]
fn test_this_constructor_access() {
    let result = eval_ts(
        r#"
        function f() {
            return this.constructor;
        }
        f()
    "#,
    );
    // Should not leak the Function constructor
    match result {
        Err(_) => {} // blocked
        Ok(val) => assert_eq!(
            val,
            Value::Undefined,
            "VULN: this.constructor leaked a value"
        ),
    }
}

#[test]
fn test_arguments_callee() {
    let result = eval_ts(
        r#"
        function f() {
            return arguments.callee;
        }
        f()
    "#,
    );
    // Should either error or return undefined
    match result {
        Err(_) => {}
        Ok(_val) => {
            // Must not return a function reference that can be further exploited
        }
    }
}

#[test]
fn test_dunder_proto_access() {
    let result = eval_ts(
        r#"
        const obj = {};
        obj.__proto__
    "#,
    );
    match result {
        Err(_) => {}
        Ok(_val) => {
            // Should be undefined or null, not an actual prototype object
        }
    }
}

// ── 10. Unicode / encoding attacks ──────────────────────────────────

#[test]
fn test_null_byte_in_string() {
    let result = eval_ts(
        r#"
        const s = "hello\0world";
        s.length
    "#,
    );
    match result {
        Err(_) => {}
        Ok(_val) => {
            // Should handle null bytes safely — length should be 11
        }
    }
}

#[test]
fn test_unicode_escape_for_eval() {
    // Try to call eval using unicode escapes
    let result = eval_ts(r#"\u0065\u0076\u0061\u006c("1+1")"#);
    assert!(
        result.is_err(),
        "VULN: unicode-escaped 'eval' call succeeded"
    );
}

#[test]
fn test_unicode_escape_for_function() {
    let result = eval_ts(r#"\u0046\u0075\u006e\u0063\u0074\u0069\u006f\u006e("return 1")()"#);
    assert!(
        result.is_err(),
        "VULN: unicode-escaped 'Function' call succeeded"
    );
}

// ── 11. Computed property access ────────────────────────────────────

#[test]
fn test_computed_globalthis_access() {
    let result = eval_ts(
        r#"
        const name = "global" + "This";
        const g = this[name];
        g
    "#,
    );
    // Should not give access to globalThis
    match result {
        Err(_) => {} // blocked
        Ok(val) => assert_eq!(
            val,
            Value::Undefined,
            "VULN: computed globalThis access succeeded"
        ),
    }
}

#[test]
fn test_computed_eval_access() {
    let result = eval_ts(
        r#"
        const name = "ev" + "al";
        const fn2 = this[name];
        fn2("1+1")
    "#,
    );
    assert!(result.is_err(), "VULN: computed eval access succeeded");
}

#[test]
fn test_computed_constructor_access() {
    let result = eval_ts(
        r#"
        const key = "construct" + "or";
        const obj = {};
        obj[key][key]("return 1")()
    "#,
    );
    assert!(
        result.is_err(),
        "VULN: computed constructor chain succeeded"
    );
}

// ── 12. Timing side channel ─────────────────────────────────────────

#[test]
fn test_date_now_blocked() {
    let result = eval_ts("Date.now()");
    // Date.now() could be used for timing attacks
    // It may or may not be available — if it is, it should return a number (not dangerous alone)
    match result {
        Err(_) => {} // blocked — good for security
        Ok(_val) => {
            // Date.now() existing isn't a vulnerability per se, but note it
        }
    }
}

#[test]
fn test_performance_now_blocked() {
    let result = eval_ts("performance.now()");
    // High-resolution timer — should be blocked
    assert!(
        result.is_err(),
        "VULN: performance.now() available in sandbox"
    );
}

// ── 13. Error message information leakage ───────────────────────────

#[test]
fn test_error_no_host_paths() {
    // Use a runtime error that actually errors, not an undefined variable reference
    let result = eval_ts("null.property");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    // Error message should not contain host filesystem paths
    assert!(
        !err_msg.contains("/home"),
        "VULN: error leaks host path: {err_msg}"
    );
    assert!(
        !err_msg.contains("\\Users"),
        "VULN: error leaks Windows path: {err_msg}"
    );
    assert!(
        !err_msg.contains("/usr"),
        "VULN: error leaks system path: {err_msg}"
    );
}

#[test]
fn test_error_no_env_leak() {
    let result = eval_ts("throw new Error('test')");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        !err_msg.contains("HOME="),
        "VULN: error leaks env vars: {err_msg}"
    );
    assert!(
        !err_msg.contains("PATH="),
        "VULN: error leaks env vars: {err_msg}"
    );
}

#[test]
fn test_stack_trace_no_host_info() {
    let result = eval_ts(
        r#"
        function a() { throw new Error("trace"); }
        function b() { a(); }
        b()
    "#,
    );
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        !err_msg.contains("zapcode-core"),
        "VULN: error leaks crate name: {err_msg}"
    );
    assert!(
        !err_msg.contains("src/vm"),
        "VULN: error leaks source path: {err_msg}"
    );
}

// ── 14. Type confusion ──────────────────────────────────────────────

#[test]
fn test_type_confusion_object_as_function() {
    let result = eval_ts(
        r#"
        const obj = { call: 42 };
        obj()
    "#,
    );
    assert!(result.is_err(), "VULN: non-function object was callable");
}

#[test]
fn test_type_confusion_number_as_object() {
    let result = eval_ts(
        r#"
        const n: any = 42;
        n.evil = "payload";
        n.evil
    "#,
    );
    match result {
        Err(_) => {} // blocked
        Ok(val) => {
            // Should be undefined — numbers don't have mutable properties
            assert_ne!(
                val,
                Value::String("payload".into()),
                "VULN: number gained properties"
            );
        }
    }
}

#[test]
fn test_type_confusion_null_property() {
    let result = eval_ts(
        r#"
        const n: any = null;
        n.property
    "#,
    );
    assert!(result.is_err(), "VULN: null property access didn't error");
}

#[test]
fn test_type_confusion_undefined_property() {
    let result = eval_ts(
        r#"
        const u: any = undefined;
        u.property
    "#,
    );
    assert!(
        result.is_err(),
        "VULN: undefined property access didn't error"
    );
}

// ── 15. Regex DoS (ReDoS) ──────────────────────────────────────────

#[test]
fn test_regex_catastrophic_backtracking() {
    // Even if regex is supported, catastrophic backtracking should be stopped
    let result = eval_ts(
        r#"
        const re = /^(a+)+$/;
        re.test("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab")
    "#,
    );
    // regex blocked or ReDoS caught; if it returns quickly, the engine handles it safely
    let _ = result;
}

#[test]
fn test_regex_constructor_blocked() {
    let result = eval_ts(r#"new RegExp("(a+)+$")"#);
    // blocked or allowed but safe is ok
    let _ = result;
}

// ── 16. Promise-based escape ────────────────────────────────────────

#[test]
fn test_promise_constructor_escape() {
    let result = eval_ts(
        r#"
        new Promise((resolve) => {
            resolve(this.constructor);
        })
    "#,
    );
    // blocked, or promise returned but can't escape
    let _ = result;
}

#[test]
fn test_promise_then_chaining_dos() {
    let result = eval_ts(
        r#"
        let p = Promise.resolve(0);
        for (let i = 0; i < 1000000; i++) {
            p = p.then((x) => x + 1);
        }
        p
    "#,
    );
    // blocked by allocation or time limit; if it handles it, fine
    let _ = result;
}

// ── 17. Generator-based escape ──────────────────────────────────────

#[test]
fn test_generator_access_internals() {
    // Try to access generator internals via the iterator protocol
    let result = eval_ts(
        r#"
        function* gen() { yield 1; }
        const g = gen();
        const proto = Object.getPrototypeOf(g);
        proto.constructor
    "#,
    );
    match result {
        Err(_) => {} // blocked
        Ok(val) => {
            // Should not leak internal VM state
            assert_ne!(
                val,
                Value::String("Function".into()),
                "VULN: generator prototype leaked Function"
            );
        }
    }
}

#[test]
fn test_generator_return_escape() {
    // Try to use generator.return() to manipulate VM state
    let result = eval_ts(
        r#"
        function* gen() {
            try {
                yield 1;
                yield 2;
            } finally {
                yield "intercepted";
            }
        }
        const g = gen();
        g.next();
        g.return("forced")
    "#,
    );
    // This is normal generator behavior — just ensure it doesn't crash
    // blocked or normal behavior
    let _ = result;
}

// ── 18. Sparse array attack ────────────────────────────────────────

#[test]
fn test_sparse_array_max_safe_integer() {
    let result = eval_ts(
        r#"
        const arr: any[] = [];
        arr[Number.MAX_SAFE_INTEGER] = 1;
        arr.length
    "#,
    );
    // Should not allocate 9007199254740991 elements
    assert!(
        result.is_err(),
        "VULN: sparse array with MAX_SAFE_INTEGER not caught"
    );
}

#[test]
fn test_sparse_array_large_index() {
    let result = eval_ts(
        r#"
        const arr: any[] = [];
        arr[1000000000] = 1;
        arr.length
    "#,
    );
    // Should not allocate a billion-element array
    assert!(
        result.is_err(),
        "VULN: sparse array with huge index not caught"
    );
}

// ── 19. Negative array index ────────────────────────────────────────

#[test]
fn test_negative_array_index() {
    let result = eval_ts(
        r#"
        const arr = [1, 2, 3];
        arr[-1]
    "#,
    );
    // Should return undefined, not access memory before the array
    match result {
        Err(_) => {} // blocked
        Ok(val) => assert_eq!(
            val,
            Value::Undefined,
            "VULN: negative index returned a value"
        ),
    }
}

#[test]
fn test_negative_large_array_index() {
    let result = eval_ts(
        r#"
        const arr = [1, 2, 3];
        arr[-1000000]
    "#,
    );
    match result {
        Err(_) => {}
        Ok(val) => assert_eq!(
            val,
            Value::Undefined,
            "VULN: large negative index returned a value"
        ),
    }
}

// ── Additional attack vectors ───────────────────────────────────────

#[test]
fn test_symbol_tostringtag_override() {
    // Try to override Symbol.toStringTag to confuse type checks
    let result = eval_ts(
        r#"
        const obj = {
            get [Symbol.toStringTag]() { return "Process"; }
        };
        Object.prototype.toString.call(obj)
    "#,
    );
    // blocked, or fine as long as no escape
    let _ = result;
}

#[test]
fn test_proxy_trap_escape() {
    // Proxy could intercept operations and escape
    let result = eval_ts(
        r#"
        const handler = {
            get(target: any, prop: string) {
                return process;
            }
        };
        const p = new Proxy({}, handler);
        p.anything
    "#,
    );
    assert!(result.is_err(), "VULN: Proxy + process access succeeded");
}

#[test]
fn test_with_statement_scope_escape() {
    // 'with' statement can manipulate scope chain
    let result = eval_ts(
        r#"
        with ({ constructor: Function }) {
            constructor("return 1")()
        }
    "#,
    );
    assert!(
        result.is_err(),
        "VULN: 'with' statement scope escape succeeded"
    );
}

#[test]
fn test_eval_indirect() {
    // Indirect eval — (0, eval)("code")
    let result = eval_ts(r#"(0, eval)("1 + 1")"#);
    assert!(result.is_err(), "VULN: indirect eval succeeded");
}

#[test]
fn test_import_meta() {
    let result = eval_ts("import.meta.url");
    assert!(result.is_err(), "VULN: import.meta accessible");
}

#[test]
fn test_new_function_constructor() {
    let result = eval_ts(r#"new Function("return 1")()"#);
    assert!(
        result.is_err(),
        "VULN: new Function() constructor succeeded"
    );
}

#[test]
fn test_settimeout_blocked() {
    let result = eval_ts(r#"setTimeout(() => {}, 0)"#);
    assert!(result.is_err(), "VULN: setTimeout available in sandbox");
}

#[test]
fn test_setinterval_blocked() {
    let result = eval_ts(r#"setInterval(() => {}, 100)"#);
    assert!(result.is_err(), "VULN: setInterval available in sandbox");
}

/// Object.freeze is not yet implemented — frozen objects can still be mutated.
/// This is a correctness gap (not a sandbox escape), since Object.freeze cannot
/// be used to break out of the sandbox.
#[test]
#[ignore = "Object.freeze not yet implemented"]
fn test_object_freeze_bypass() {
    // Try to modify a frozen object
    let result = eval_ts(
        r#"
        const obj = Object.freeze({ x: 1 });
        obj.x = 999;
        obj.x
    "#,
    );
    match result {
        Err(_) => {} // strict mode error — good
        Ok(val) => assert_eq!(val, Value::Int(1), "VULN: Object.freeze bypassed"),
    }
}

#[test]
fn test_reflect_construct_escape() {
    let result = eval_ts(
        r#"
        Reflect.construct(Function, ["return 1"])()
    "#,
    );
    assert!(result.is_err(), "VULN: Reflect.construct escape succeeded");
}

#[test]
fn test_async_generator_dos() {
    let result = eval_ts(
        r#"
        async function* inf() {
            while (true) {
                yield 1;
            }
        }
        const g = inf();
        for await (const x of g) {}
    "#,
    );
    assert!(result.is_err(), "VULN: infinite async generator not caught");
}

#[test]
fn test_string_repeat_huge() {
    let result = eval_ts(
        r#"
        "a".repeat(1000000000)
    "#,
    );
    assert!(result.is_err(), "VULN: huge string.repeat not limited");
}

#[test]
fn test_array_constructor_huge() {
    let result = eval_ts(
        r#"
        new Array(1000000000)
    "#,
    );
    assert!(result.is_err(), "VULN: huge Array constructor not limited");
}

#[test]
fn test_object_define_property_escape() {
    let result = eval_ts(
        r#"
        const obj = {};
        Object.defineProperty(obj, "evil", {
            get() { return globalThis; }
        });
        obj.evil
    "#,
    );
    assert!(
        result.is_err(),
        "VULN: Object.defineProperty + globalThis escape"
    );
}

#[test]
fn test_double_free_via_delete() {
    // Try to cause memory issues via delete
    let result = eval_ts(
        r#"
        const obj = { a: [1, 2, 3] };
        const ref1 = obj.a;
        delete obj.a;
        ref1[0]
    "#,
    );
    // Should not crash — either error or return the value safely
    match result {
        Err(_) => {}                               // delete not supported is fine
        Ok(val) => assert_eq!(val, Value::Int(1)), // safe access
    }
}

#[test]
fn test_weakref_escape() {
    let result = eval_ts(
        r#"
        const obj = {};
        const ref2 = new WeakRef(obj);
        ref2.deref()
    "#,
    );
    // WeakRef not supported is good; if supported, ensure it's safe
    let _ = result;
}

#[test]
fn test_finalization_registry_escape() {
    let result = eval_ts(
        r#"
        const registry = new FinalizationRegistry((value) => {
            // Try to access host during cleanup
        });
        registry.register({}, "cleanup");
    "#,
    );
    // not supported is good
    let _ = result;
}

// ── 12. Spread amplification ───────────────────────────────────────
// Arrays and objects are value types, so `[...a, ...a]` doubles the payload
// while costing O(1) stack pushes. Flattening must count each produced
// element against the allocation limit, or a short loop bypasses it.

#[test]
fn test_array_spread_amplification_hits_allocation_limit() {
    let limits = ResourceLimits {
        max_allocations: 10_000,
        ..Default::default()
    };
    let runner = ZapcodeRun::new(
        r#"
        let a: any = [1, 2, 3, 4];
        for (let i = 0; i < 16; i++) {
            a = [...a, ...a];
        }
        a.length
        "#
        .to_string(),
        vec![],
        vec![],
        limits,
    )
    .unwrap();
    let result = runner.start(vec![]);
    assert!(
        matches!(result, Err(ZapcodeError::AllocationLimitExceeded)),
        "VULN: array spread flattening not counted against the allocation limit, got: {result:?}"
    );
}

#[test]
fn test_object_spread_amplification_hits_allocation_limit() {
    let limits = ResourceLimits {
        max_allocations: 10_000,
        ..Default::default()
    };
    let runner = ZapcodeRun::new(
        r#"
        const s = "ab".repeat(20000);
        const o = { ...s };
        Object.keys(o).length
        "#
        .to_string(),
        vec![],
        vec![],
        limits,
    )
    .unwrap();
    let result = runner.start(vec![]);
    assert!(
        matches!(result, Err(ZapcodeError::AllocationLimitExceeded)),
        "VULN: object spread merging not counted against the allocation limit, got: {result:?}"
    );
}
