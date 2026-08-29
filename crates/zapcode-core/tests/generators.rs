use zapcode_core::{ResourceLimits, Value, ZapcodeError, ZapcodeRun};

/// Helper to run TS code and get stdout + value
fn eval_with_output(code: &str) -> (Value, String) {
    zapcode_core::vm::eval_ts_with_output(code).unwrap()
}

#[test]
fn test_generator_next_respects_stack_limit() {
    let runner = ZapcodeRun::new(
        r#"
        function* values() { yield 1; }
        values().next()
        "#
        .to_string(),
        Vec::new(),
        Vec::new(),
        ResourceLimits {
            max_stack_depth: 0,
            ..ResourceLimits::default()
        },
    )
    .unwrap();

    assert!(matches!(
        runner.start(Vec::new()),
        Err(ZapcodeError::StackOverflow(_))
    ));
}

#[test]
fn test_resumed_generator_respects_stack_limit() {
    let runner = ZapcodeRun::new(
        r#"
        function* values() { yield 1; yield 2; }
        const iterator = values();
        iterator.next();
        function resume() { return iterator.next(); }
        resume()
        "#
        .to_string(),
        Vec::new(),
        Vec::new(),
        ResourceLimits {
            max_stack_depth: 1,
            ..ResourceLimits::default()
        },
    )
    .unwrap();

    assert!(matches!(
        runner.start(Vec::new()),
        Err(ZapcodeError::StackOverflow(_))
    ));
}

#[test]
fn test_basic_generator_yield() {
    let (val, _) = eval_with_output(
        r#"
        function* simple() {
            yield 1;
            yield 2;
            yield 3;
        }
        const g = simple();
        const a = g.next();
        const b = g.next();
        const c = g.next();
        const d = g.next();
        [a.value, a.done, b.value, b.done, c.value, c.done, d.value, d.done]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Int(1));
            assert_eq!(items[1], Value::Bool(false));
            assert_eq!(items[2], Value::Int(2));
            assert_eq!(items[3], Value::Bool(false));
            assert_eq!(items[4], Value::Int(3));
            assert_eq!(items[5], Value::Bool(false));
            assert_eq!(items[6], Value::Undefined);
            assert_eq!(items[7], Value::Bool(true));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_with_return() {
    let (val, _) = eval_with_output(
        r#"
        function* withReturn() {
            yield 1;
            return 42;
        }
        const g = withReturn();
        const a = g.next();
        const b = g.next();
        const c = g.next();
        [a.value, a.done, b.value, b.done, c.value, c.done]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Int(1));
            assert_eq!(items[1], Value::Bool(false));
            assert_eq!(items[2], Value::Int(42));
            assert_eq!(items[3], Value::Bool(true));
            assert_eq!(items[4], Value::Undefined);
            assert_eq!(items[5], Value::Bool(true));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_with_args() {
    let (val, _) = eval_with_output(
        r#"
        function* range(start, end) {
            for (let i = start; i < end; i++) {
                yield i;
            }
        }
        const g = range(0, 3);
        const a = g.next();
        const b = g.next();
        const c = g.next();
        const d = g.next();
        [a.value, a.done, b.value, b.done, c.value, c.done, d.value, d.done]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Int(0));
            assert_eq!(items[1], Value::Bool(false));
            assert_eq!(items[2], Value::Int(1));
            assert_eq!(items[3], Value::Bool(false));
            assert_eq!(items[4], Value::Int(2));
            assert_eq!(items[5], Value::Bool(false));
            assert_eq!(items[6], Value::Undefined);
            assert_eq!(items[7], Value::Bool(true));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_receive_values() {
    let (val, _) = eval_with_output(
        r#"
        function* adder() {
            let sum = 0;
            while (true) {
                const x = yield sum;
                sum += x;
            }
        }
        const g = adder();
        const a = g.next();
        const b = g.next(10);
        const c = g.next(20);
        [a.value, b.value, c.value]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Int(0));
            assert_eq!(items[1], Value::Int(10));
            assert_eq!(items[2], Value::Int(30));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_done_state() {
    let (val, _) = eval_with_output(
        r#"
        function* once() {
            yield 42;
        }
        const g = once();
        g.next();
        g.next();
        const r1 = g.next();
        const r2 = g.next();
        [r1.value, r1.done, r2.value, r2.done]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Undefined);
            assert_eq!(items[1], Value::Bool(true));
            assert_eq!(items[2], Value::Undefined);
            assert_eq!(items[3], Value::Bool(true));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_for_of() {
    let (_, stdout) = eval_with_output(
        r#"
        function* range(start, end) {
            for (let i = start; i < end; i++) {
                yield i;
            }
        }
        for (const x of range(0, 3)) {
            console.log(x);
        }
        "#,
    );
    assert_eq!(stdout.trim(), "0\n1\n2");
}

#[test]
fn test_generator_with_for_loop() {
    let (val, _) = eval_with_output(
        r#"
        function* countdown(n) {
            for (let i = n; i > 0; i--) {
                yield i;
            }
        }
        const g = countdown(3);
        const a = g.next();
        const b = g.next();
        const c = g.next();
        const d = g.next();
        [a.value, b.value, c.value, d.done]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Int(3));
            assert_eq!(items[1], Value::Int(2));
            assert_eq!(items[2], Value::Int(1));
            assert_eq!(items[3], Value::Bool(true));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_yield_undefined() {
    let (val, _) = eval_with_output(
        r#"
        function* gen() {
            yield;
            yield;
        }
        const g = gen();
        const a = g.next();
        const b = g.next();
        [a.value, a.done, b.value, b.done]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Undefined);
            assert_eq!(items[1], Value::Bool(false));
            assert_eq!(items[2], Value::Undefined);
            assert_eq!(items[3], Value::Bool(false));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_with_while_loop() {
    let (val, _) = eval_with_output(
        r#"
        function* fibonacci() {
            let a = 0;
            let b = 1;
            while (true) {
                yield a;
                const temp = a;
                a = b;
                b = temp + b;
            }
        }
        const g = fibonacci();
        const r0 = g.next().value;
        const r1 = g.next().value;
        const r2 = g.next().value;
        const r3 = g.next().value;
        const r4 = g.next().value;
        const r5 = g.next().value;
        const r6 = g.next().value;
        [r0, r1, r2, r3, r4, r5, r6]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items.len(), 7);
            assert_eq!(items[0], Value::Int(0));
            assert_eq!(items[1], Value::Int(1));
            assert_eq!(items[2], Value::Int(1));
            assert_eq!(items[3], Value::Int(2));
            assert_eq!(items[4], Value::Int(3));
            assert_eq!(items[5], Value::Int(5));
            assert_eq!(items[6], Value::Int(8));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_multiple_generator_instances() {
    let (val, _) = eval_with_output(
        r#"
        function* counter() {
            let n = 0;
            while (true) {
                yield n;
                n += 1;
            }
        }
        const g1 = counter();
        const g2 = counter();
        const a = g1.next().value;
        const b = g1.next().value;
        const c = g2.next().value;
        const d = g1.next().value;
        [a, b, c, d]
        "#,
    );
    match val {
        Value::Array(items) => {
            assert_eq!(items[0], Value::Int(0));
            assert_eq!(items[1], Value::Int(1));
            assert_eq!(items[2], Value::Int(0));
            assert_eq!(items[3], Value::Int(2));
        }
        other => panic!("Expected array, got {:?}", other),
    }
}

#[test]
fn test_generator_for_of_with_console_log() {
    let (_, stdout) = eval_with_output(
        r#"
        function* nums() {
            yield 10;
            yield 20;
            yield 30;
        }
        for (const n of nums()) {
            console.log(n);
        }
        "#,
    );
    assert_eq!(stdout.trim(), "10\n20\n30");
}

#[test]
fn test_generator_expression_result() {
    let (val, _) = eval_with_output(
        r#"
        function* gen() {
            const x = yield 1;
            yield x + 10;
        }
        const g = gen();
        g.next();
        const r = g.next(5);
        r.value
        "#,
    );
    assert_eq!(val, Value::Int(15));
}
