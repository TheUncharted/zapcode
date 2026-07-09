use zapcode_core::vm::eval_ts;
use zapcode_core::Value;

#[test]
fn test_if_true() {
    let result = eval_ts("let x = 0; if (true) { x = 1; } x").unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_if_false() {
    let result = eval_ts("let x = 0; if (false) { x = 1; } x").unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_if_else() {
    let result = eval_ts("let x = 0; if (false) { x = 1; } else { x = 2; } x").unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_while_loop() {
    let result = eval_ts("let sum = 0; let i = 1; while (i <= 10) { sum += i; i++; } sum").unwrap();
    assert_eq!(result, Value::Int(55));
}

#[test]
fn test_for_loop() {
    let result = eval_ts("let sum = 0; for (let i = 0; i < 5; i++) { sum += i; } sum").unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_break_in_loop() {
    let result = eval_ts("let x = 0; while (true) { x++; if (x === 5) { break; } } x").unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_continue_in_loop() {
    let result = eval_ts(
        "let sum = 0; for (let i = 0; i < 10; i++) { if (i % 2 === 0) { continue; } sum += i; } sum",
    )
    .unwrap();
    assert_eq!(result, Value::Int(25));
}

#[test]
fn test_nested_if() {
    let result = eval_ts("let x = 0; if (true) { if (true) { x = 42; } } x").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_logical_and() {
    let result = eval_ts("true && 42").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_logical_or() {
    let result = eval_ts("false || 42").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_nullish_coalescing() {
    let result = eval_ts("null ?? 42").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_nullish_coalescing_defined() {
    let result = eval_ts("0 ?? 42").unwrap();
    assert_eq!(result, Value::Int(0));
}

// ── switch (regression: bare `break` looped forever → allocation blowup) ─────

#[test]
fn test_switch_basic_match_and_break() {
    let r = eval_ts(
        "let r = 'none'; switch (2) { case 1: r = 'one'; break; case 2: r = 'two'; break; } r",
    )
    .unwrap();
    assert_eq!(r, Value::String("two".into()));
}

#[test]
fn test_switch_default() {
    let r = eval_ts("let r = 'x'; switch (9) { case 1: r = 'one'; break; default: r = 'def'; } r")
        .unwrap();
    assert_eq!(r, Value::String("def".into()));
}

#[test]
fn test_switch_fallthrough() {
    let r = eval_ts(
        "let r = 0; switch (1) { case 1: r += 1; case 2: r += 10; break; case 3: r += 100; } r",
    )
    .unwrap();
    assert_eq!(r, Value::Int(11));
}

#[test]
fn test_switch_break_inside_loop_breaks_switch_only() {
    // break exits the switch (not the loop): n=1 adds 1, n=2 hits default (+100);
    // the loop still runs both iterations and the trailing `t += 1000` each time.
    let src = "let t = 0; for (const n of [1, 2]) { switch (n) { case 1: t += 1; break; default: t += 100; } t += 1000; } t";
    let r = eval_ts(src).unwrap();
    assert_eq!(r, Value::Int(2101));
}
