use zapcode_core::vm::eval_ts;
use zapcode_core::Value;

#[test]
fn test_array_literal() {
    let result = eval_ts("[1, 2, 3]").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Int(1));
            assert_eq!(arr[1], Value::Int(2));
            assert_eq!(arr[2], Value::Int(3));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_length() {
    let result = eval_ts("const arr = [1, 2, 3]; arr.length").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_object_literal() {
    let result = eval_ts("const obj = { x: 1, y: 2 }; obj.x + obj.y").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_nested_object() {
    let result = eval_ts("const obj = { a: { b: 42 } }; obj.a.b").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_computed_property_access() {
    let result = eval_ts("const arr = [10, 20, 30]; arr[1]").unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_object_shorthand() {
    let result = eval_ts("const x = 42; const obj = { x }; obj.x").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_destructuring_object() {
    let result = eval_ts("const obj = { a: 1, b: 2 }; const { a, b } = obj; a + b").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_destructuring_array() {
    let result = eval_ts("const arr = [10, 20]; const [a, b] = arr; a + b").unwrap();
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_template_literal() {
    let result = eval_ts("const name = \"world\"; `hello ${name}`").unwrap();
    assert_eq!(result, Value::String("hello world".into()));
}

#[test]
fn test_string_length() {
    let result = eval_ts("\"hello\".length").unwrap();
    assert_eq!(result, Value::Int(5));
}

// --- Trailing object literal auto-detection ---

#[test]
fn test_trailing_object_shorthand() {
    let result = eval_ts("const a = 1\nconst b = 2\n{ a, b }").unwrap();
    match result {
        Value::Object(map) => {
            assert_eq!(map.get("a"), Some(&Value::Int(1)));
            assert_eq!(map.get("b"), Some(&Value::Int(2)));
        }
        other => panic!("expected object, got {:?}", other),
    }
}

#[test]
fn test_trailing_object_key_value() {
    let result = eval_ts("const x = 10\n{ value: x }").unwrap();
    match result {
        Value::Object(map) => {
            assert_eq!(map.get("value"), Some(&Value::Int(10)));
        }
        other => panic!("expected object, got {:?}", other),
    }
}

#[test]
fn test_trailing_object_mixed() {
    let result = eval_ts("const name = \"hello\"\nconst age = 30\n{ name, years: age }").unwrap();
    match result {
        Value::Object(map) => {
            assert_eq!(map.get("name"), Some(&Value::String("hello".into())));
            assert_eq!(map.get("years"), Some(&Value::Int(30)));
        }
        other => panic!("expected object, got {:?}", other),
    }
}

#[test]
fn test_trailing_object_with_parens_still_works() {
    let result = eval_ts("const a = 1;\n({ a })").unwrap();
    match result {
        Value::Object(map) => {
            assert_eq!(map.get("a"), Some(&Value::Int(1)));
        }
        other => panic!("expected object, got {:?}", other),
    }
}

// --- Edge cases: things that should NOT be wrapped ---

#[test]
fn test_block_assignment_not_wrapped() {
    // `{ x = 5 }` is a block with assignment, not an object
    let result = eval_ts("let x = 0\n{ x = 5 }\nx").unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_if_else_block_not_wrapped() {
    let result = eval_ts("const x = true\nif (x) { 1 } else { 2 }").unwrap();
    // if/else is a statement — must not be mistaken for an object literal
    assert!(
        !matches!(result, Value::Object(_)),
        "if/else block was incorrectly wrapped as object literal, got {:?}",
        result
    );
}

#[test]
fn test_arrow_fn_body_not_wrapped() {
    let result = eval_ts("const f = () => { return 42 }\nf()").unwrap();
    assert_eq!(result, Value::Int(42));
}

// --- Edge cases: things that SHOULD be wrapped ---

#[test]
fn test_trailing_object_single_prop() {
    let result = eval_ts("const x = 42\n{ value: x }").unwrap();
    match result {
        Value::Object(map) => {
            assert_eq!(map.get("value"), Some(&Value::Int(42)));
        }
        other => panic!("expected object, got {:?}", other),
    }
}

#[test]
fn test_trailing_object_after_semicolon() {
    let result = eval_ts("const a = 1; const b = 2;\n{ a, b }").unwrap();
    match result {
        Value::Object(map) => {
            assert_eq!(map.get("a"), Some(&Value::Int(1)));
            assert_eq!(map.get("b"), Some(&Value::Int(2)));
        }
        other => panic!("expected object, got {:?}", other),
    }
}

// --- Bug fix: keyword+paren+block constructs with object literal args ---

#[test]
fn test_if_block_with_object_arg() {
    let result = eval_ts("if (true) { Promise.resolve({ a: 1 }); }").unwrap();
    assert_eq!(result, Value::Undefined);
}

#[test]
fn test_for_block_with_object_arg() {
    let result = eval_ts("let sum = 0; for (let i = 0; i < 3; i++) { sum += i; } sum").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_while_block_with_object_arg() {
    let result = eval_ts("let x = 0; while (x < 3) { x++; } x").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_catch_block_with_object_arg() {
    let result = eval_ts(
        "let caught = false; try { throw new Error('test'); } catch (e) { caught = true; } caught",
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_nested_if_block_with_object_arg() {
    let result = eval_ts("let x = 0; if (true) { if (true) { x = 42; } } x").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_else_if_block_with_object_arg() {
    let result = eval_ts("let x = 0; if (false) { x = 1; } else if (true) { x = 2; } x").unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_for_of_block_with_object_arg() {
    let result = eval_ts("let sum = 0; for (const x of [1, 2, 3]) { sum += x; } sum").unwrap();
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_if_block_with_await_and_object_arg() {
    let result = eval_ts("if (true) { await Promise.resolve({ a: 1 }); }").unwrap();
    assert_eq!(result, Value::Undefined);
}

#[test]
fn test_if_block_with_user_function_and_object_arg() {
    let result = eval_ts("function f(x){ return x; }\nif (true) { f({ a: 1 }); }").unwrap();
    assert_eq!(result, Value::Undefined);
}

// ── spread (regression: were silently-wrong / crashing) ──────────────────────

#[test]
fn test_array_spread_flattens() {
    // (Value's PartialEq is JS ===, so arrays compare by reference — assert via join.)
    let r = eval_ts("const a = [1, 2]; const b = [...a, 3]; b.join(',')").unwrap();
    assert_eq!(r, Value::String("1,2,3".into()));
}

#[test]
fn test_array_spread_middle_and_multiple() {
    let r = eval_ts("const a = [1]; const b = [2, 3]; [0, ...a, ...b, 4].join(',')").unwrap();
    assert_eq!(r, Value::String("0,1,2,3,4".into()));
}

#[test]
fn test_object_spread() {
    let r = eval_ts("const a = { x: 1 }; const b = { ...a, y: 2 }; b.x + b.y").unwrap();
    assert_eq!(r, Value::Int(3));
}

#[test]
fn test_object_spread_overrides_later_wins() {
    let r = eval_ts("const o = { ...{ a: 1, b: 2 }, b: 3 }; o.b").unwrap();
    assert_eq!(r, Value::Int(3));
}

#[test]
fn test_array_spread_non_iterable_errors() {
    let err = eval_ts("const x = 5; [...x]").unwrap_err();
    assert!(err.to_string().contains("not iterable"), "got: {err}");
}

#[test]
fn test_array_spread_empty_source() {
    assert_eq!(eval_ts("[...[]].length").unwrap(), Value::Int(0));
}

#[test]
fn test_object_spread_empty_source() {
    assert_eq!(
        eval_ts("Object.keys({ ...{} }).length").unwrap(),
        Value::Int(0)
    );
}

#[test]
fn test_object_spread_string_uses_char_index_keys() {
    assert_eq!(
        eval_ts(r#"const o = { ..."ab" }; o["1"]"#).unwrap(),
        Value::String("b".into())
    );
}
