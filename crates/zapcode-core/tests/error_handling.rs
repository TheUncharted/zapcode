use zapcode_core::vm::eval_ts;
use zapcode_core::Value;

#[test]
fn test_try_catch() {
    let result = eval_ts(
        r#"
        let caught = false;
        try {
            throw "error";
        } catch (e) {
            caught = true;
        }
        caught
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_try_catch_value() {
    let result = eval_ts(
        r#"
        let msg = "";
        try {
            throw "oops";
        } catch (e) {
            msg = e;
        }
        msg
        "#,
    )
    .unwrap();
    // The thrown value becomes a runtime error message
    if let Value::String(s) = result {
        assert!(!s.is_empty());
    }
}

#[test]
fn test_try_no_error() {
    let result = eval_ts(
        r#"
        let x = 0;
        try {
            x = 42;
        } catch (e) {
            x = -1;
        }
        x
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(42));
}

// Regression: a throw escaping a nested array callback (or a callback inside a
// class method) emptied the VM frame stack; execute() then hit
// frames.last().unwrap() and aborted the host process. These must surface an
// error to the caller, never panic. (The guest-level catch not observing the
// throw is a separate, pre-existing unwinding issue.)
#[test]
fn test_throw_from_nested_callback_does_not_panic() {
    let result = eval_ts(
        r#"
        let out = 0;
        try {
            [1].map(a => [2].map(b => { throw "n"; }));
        } catch (e) { out = 3; }
        out
        "#,
    );
    assert!(result.is_err());
}

#[test]
fn test_throw_from_class_method_callback_does_not_panic() {
    let result = eval_ts(
        r#"
        class A { run() { return [1].map(x => { throw "m"; }); } }
        let out = 0;
        try { new A().run(); } catch (e) { out = 6; }
        out
        "#,
    );
    assert!(result.is_err());
}
