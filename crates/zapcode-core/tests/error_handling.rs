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
    )
    .unwrap();
    assert_eq!(result, Value::Int(3));
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
    )
    .unwrap();
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_return_inside_try_does_not_leave_stale_handler() {
    let result = eval_ts(
        r#"
        function seed() {
            try { return 1; } catch (e) {}
        }
        seed();

        let out = 0;
        try {
            [1].map(a => [2].map(b => { throw "n"; }));
        } catch (e) { out = 9; }
        out
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(9));
}

#[test]
fn test_generator_return_inside_try_does_not_leave_stale_handler() {
    let result = eval_ts(
        r#"
        function* seed() {
            try { return 1; } catch (e) {}
        }
        seed().next();

        let out = 0;
        try {
            [1].map(a => [2].map(b => { throw "n"; }));
        } catch (e) { out = 11; }
        out
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(11));
}

#[test]
fn test_generator_throw_reaches_outer_try_handler() {
    let result = eval_ts(
        r#"
        function* fail() { throw "generator error"; }
        let out = 0;
        try { fail().next(); } catch (e) { out = 13; }
        out
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(13));
}

#[test]
fn test_generator_try_handler_survives_yield() {
    let result = eval_ts(
        r#"
        function* values() {
            try {
                yield 1;
                throw "failure";
            } catch (e) {
                yield 2;
            }
        }
        const iterator = values();
        iterator.next();
        iterator.next().value
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_async_callback_throw_cancels_continuation() {
    let result = eval_ts(
        r#"
        let caught = false;
        try {
            [1].map(async () => { throw "failure"; });
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
fn test_nested_generator_try_handlers_survive_multiple_yields() {
    let result = eval_ts(
        r#"
        function* values() {
            try {
                yield 1;
                try {
                    yield 2;
                    throw "inner";
                } catch (e) {
                    yield 3;
                }
                throw "outer";
            } catch (e) {
                yield 4;
            }
        }
        const iterator = values();
        const first = iterator.next().value;
        const second = iterator.next().value;
        const third = iterator.next().value;
        const fourth = iterator.next().value;
        `${first},${second},${third},${fourth}`
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("1,2,3,4".into()));
}

#[test]
fn test_nested_async_callback_throw_preserves_outer_continuation() {
    let result = eval_ts(
        r#"
        const output = [1].map(async () => {
            let marker = 0;
            try {
                [1].map(async () => { throw "inner"; });
            } catch (e) {
                marker = 7;
            }
            return marker;
        });
        output[0]
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_break_out_of_try_removes_handler() {
    let result = eval_ts(
        r#"
        let marker = 0;
        try {
            for (let i = 0; i < 1; i++) {
                try { break; } catch (e) { marker += 1; }
            }
            null.missing;
        } catch (e) {
            marker += 10;
        }
        marker
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_continue_out_of_try_removes_handler() {
    let result = eval_ts(
        r#"
        let marker = 0;
        try {
            for (let i = 0; i < 1; i++) {
                try { continue; } catch (e) { marker += 1; }
            }
            null.missing;
        } catch (e) {
            marker += 10;
        }
        marker
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_generator_break_out_of_try_does_not_suspend_stale_handler() {
    let result = eval_ts(
        r#"
        function* values() {
            let marker = 0;
            try {
                for (let i = 0; i < 1; i++) {
                    try { break; } catch (e) { marker += 1; }
                }
                yield marker;
                throw "outer";
            } catch (e) {
                marker += 10;
            }
            yield marker;
        }
        const iterator = values();
        const first = iterator.next().value;
        const second = iterator.next().value;
        `${first},${second}`
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("0,10".into()));
}

#[test]
fn test_generator_continue_out_of_try_does_not_suspend_stale_handler() {
    let result = eval_ts(
        r#"
        function* values() {
            let marker = 0;
            try {
                for (let i = 0; i < 1; i++) {
                    try { continue; } catch (e) { marker += 1; }
                }
                yield marker;
                throw "outer";
            } catch (e) {
                marker += 10;
            }
            yield marker;
        }
        const iterator = values();
        const first = iterator.next().value;
        const second = iterator.next().value;
        `${first},${second}`
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("0,10".into()));
}
