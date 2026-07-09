use zapcode_core::vm::{eval_ts, eval_ts_with_output};
use zapcode_core::Value;

// ── Console ──────────────────────────────────────────────────────────

#[test]
fn test_console_log() {
    let (_, stdout) = eval_ts_with_output("console.log(\"hello\")").unwrap();
    assert_eq!(stdout, "hello\n");
}

#[test]
fn test_console_log_multiple_args() {
    let (_, stdout) = eval_ts_with_output("console.log(1, 2, 3)").unwrap();
    assert_eq!(stdout, "1 2 3\n");
}

#[test]
fn test_console_log_multiline() {
    let (_, stdout) = eval_ts_with_output("console.log(\"a\"); console.log(\"b\")").unwrap();
    assert_eq!(stdout, "a\nb\n");
}

// ── Math ─────────────────────────────────────────────────────────────

#[test]
fn test_math_pi() {
    let result = eval_ts("Math.PI").unwrap();
    match result {
        Value::Float(f) => assert!((f - std::f64::consts::PI).abs() < 1e-10),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_math_floor() {
    let result = eval_ts("Math.floor(4.7)").unwrap();
    assert_eq!(result, Value::Float(4.0));
}

#[test]
fn test_math_ceil() {
    let result = eval_ts("Math.ceil(4.1)").unwrap();
    assert_eq!(result, Value::Float(5.0));
}

#[test]
fn test_math_abs() {
    let result = eval_ts("Math.abs(-42)").unwrap();
    assert_eq!(result, Value::Float(42.0));
}

#[test]
fn test_math_max() {
    let result = eval_ts("Math.max(1, 5, 3)").unwrap();
    assert_eq!(result, Value::Float(5.0));
}

#[test]
fn test_math_min() {
    let result = eval_ts("Math.min(1, 5, 3)").unwrap();
    assert_eq!(result, Value::Float(1.0));
}

#[test]
fn test_math_sqrt() {
    let result = eval_ts("Math.sqrt(16)").unwrap();
    assert_eq!(result, Value::Float(4.0));
}

#[test]
fn test_math_round() {
    let result = eval_ts("Math.round(4.5)").unwrap();
    assert_eq!(result, Value::Float(5.0));
}

// ── String methods ───────────────────────────────────────────────────

#[test]
fn test_string_to_upper() {
    let result = eval_ts("\"hello\".toUpperCase()").unwrap();
    assert_eq!(result, Value::String("HELLO".into()));
}

#[test]
fn test_string_to_lower() {
    let result = eval_ts("\"HELLO\".toLowerCase()").unwrap();
    assert_eq!(result, Value::String("hello".into()));
}

#[test]
fn test_string_includes() {
    let result = eval_ts("\"hello world\".includes(\"world\")").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_starts_with() {
    let result = eval_ts("\"hello\".startsWith(\"hel\")").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_ends_with() {
    let result = eval_ts("\"hello\".endsWith(\"llo\")").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_index_of() {
    let result = eval_ts("\"hello\".indexOf(\"ll\")").unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_string_trim() {
    let result = eval_ts("\"  hello  \".trim()").unwrap();
    assert_eq!(result, Value::String("hello".into()));
}

#[test]
fn test_string_split() {
    let result = eval_ts("\"a,b,c\".split(\",\")").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::String("a".into()));
            assert_eq!(arr[1], Value::String("b".into()));
            assert_eq!(arr[2], Value::String("c".into()));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_string_replace() {
    let result = eval_ts("\"hello world\".replace(\"world\", \"rust\")").unwrap();
    assert_eq!(result, Value::String("hello rust".into()));
}

#[test]
fn test_string_repeat() {
    let result = eval_ts("\"ab\".repeat(3)").unwrap();
    assert_eq!(result, Value::String("ababab".into()));
}

#[test]
fn test_string_slice() {
    let result = eval_ts("\"hello\".slice(1, 4)").unwrap();
    assert_eq!(result, Value::String("ell".into()));
}

// ── Array methods ────────────────────────────────────────────────────

#[test]
fn test_array_includes() {
    let result = eval_ts("[1, 2, 3].includes(2)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_array_index_of() {
    let result = eval_ts("[10, 20, 30].indexOf(20)").unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_array_join() {
    let result = eval_ts("[1, 2, 3].join(\"-\")").unwrap();
    assert_eq!(result, Value::String("1-2-3".into()));
}

#[test]
fn test_array_slice() {
    let result = eval_ts("[1, 2, 3, 4, 5].slice(1, 4)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Int(2));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_concat() {
    let result = eval_ts("[1, 2].concat([3, 4])").unwrap();
    match result {
        Value::Array(arr) => assert_eq!(arr.len(), 4),
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_reverse() {
    let result = eval_ts("[1, 2, 3].reverse()").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Int(3));
            assert_eq!(arr[1], Value::Int(2));
            assert_eq!(arr[2], Value::Int(1));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

// ── JSON ─────────────────────────────────────────────────────────────

#[test]
fn test_json_stringify() {
    let result = eval_ts("JSON.stringify({a: 1, b: 2})").unwrap();
    assert_eq!(result, Value::String("{\"a\":1,\"b\":2}".into()));
}

#[test]
fn test_json_parse() {
    let result = eval_ts("JSON.parse('{\"x\":42}').x").unwrap();
    assert_eq!(result, Value::Int(42));
}

// ── Object static methods ────────────────────────────────────────────

#[test]
fn test_object_keys() {
    let result = eval_ts("Object.keys({a: 1, b: 2, c: 3})").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::String("a".into()));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_object_values() {
    let result = eval_ts("Object.values({a: 1, b: 2})").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], Value::Int(1));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_is_array() {
    let result = eval_ts("Array.isArray([1, 2])").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_array_is_array_false() {
    let result = eval_ts("Array.isArray(42)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

// ── Array mutating methods ──────────────────────────────────────────

#[test]
fn test_array_push() {
    let result = eval_ts("[1, 2, 3].push(4)").unwrap();
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_array_push_multiple() {
    let result = eval_ts("[1].push(2, 3, 4)").unwrap();
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_array_pop() {
    let result = eval_ts("[1, 2, 3].pop()").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_array_pop_empty() {
    let result = eval_ts("[].pop()").unwrap();
    assert_eq!(result, Value::Undefined);
}

#[test]
fn test_array_shift() {
    let result = eval_ts("[1, 2, 3].shift()").unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_array_shift_empty() {
    let result = eval_ts("[].shift()").unwrap();
    assert_eq!(result, Value::Undefined);
}

#[test]
fn test_array_unshift() {
    let result = eval_ts("[3, 4].unshift(1, 2)").unwrap();
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_array_splice() {
    let result = eval_ts("[1, 2, 3, 4, 5].splice(1, 2)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], Value::Int(2));
            assert_eq!(arr[1], Value::Int(3));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_splice_with_insert() {
    let result = eval_ts("[1, 2, 3].splice(1, 1, 10, 20)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0], Value::Int(2));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

// ── Array callback methods ──────────────────────────────────────────

#[test]
fn test_array_map() {
    let result = eval_ts("[1, 2, 3].map((x) => x * 2)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Int(2));
            assert_eq!(arr[1], Value::Int(4));
            assert_eq!(arr[2], Value::Int(6));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_map_with_index() {
    let result = eval_ts("[10, 20, 30].map((x, i) => i)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Int(0));
            assert_eq!(arr[1], Value::Int(1));
            assert_eq!(arr[2], Value::Int(2));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_filter() {
    let result = eval_ts("[1, 2, 3, 4, 5].filter((x) => x > 3)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], Value::Int(4));
            assert_eq!(arr[1], Value::Int(5));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_filter_empty_result() {
    let result = eval_ts("[1, 2, 3].filter((x) => x > 10)").unwrap();
    match result {
        Value::Array(arr) => assert_eq!(arr.len(), 0),
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_reduce_with_init() {
    let result = eval_ts("[1, 2, 3, 4].reduce((acc, x) => acc + x, 0)").unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_array_reduce_no_init() {
    let result = eval_ts("[1, 2, 3, 4].reduce((acc, x) => acc + x)").unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_array_reduce_strings() {
    let result = eval_ts(r#"["a", "b", "c"].reduce((acc, x) => acc + x, "")"#).unwrap();
    assert_eq!(result, Value::String("abc".into()));
}

#[test]
fn test_array_foreach() {
    let (result, stdout) = eval_ts_with_output(
        r#"
        const arr = [1, 2, 3];
        arr.forEach((x) => console.log(x));
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Undefined);
    assert_eq!(stdout, "1\n2\n3\n");
}

#[test]
fn test_array_find() {
    let result = eval_ts("[1, 2, 3, 4, 5].find((x) => x > 3)").unwrap();
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_array_find_not_found() {
    let result = eval_ts("[1, 2, 3].find((x) => x > 10)").unwrap();
    assert_eq!(result, Value::Undefined);
}

#[test]
fn test_array_find_index() {
    let result = eval_ts("[1, 2, 3, 4, 5].findIndex((x) => x > 3)").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_array_find_index_not_found() {
    let result = eval_ts("[1, 2, 3].findIndex((x) => x > 10)").unwrap();
    assert_eq!(result, Value::Int(-1));
}

#[test]
fn test_array_every_true() {
    let result = eval_ts("[2, 4, 6].every((x) => x % 2 === 0)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_array_every_false() {
    let result = eval_ts("[2, 3, 6].every((x) => x % 2 === 0)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_array_some_true() {
    let result = eval_ts("[1, 3, 4].some((x) => x % 2 === 0)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_array_some_false() {
    let result = eval_ts("[1, 3, 5].some((x) => x % 2 === 0)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_array_sort_default() {
    let result = eval_ts(r#"["banana", "apple", "cherry"].sort()"#).unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::String("apple".into()));
            assert_eq!(arr[1], Value::String("banana".into()));
            assert_eq!(arr[2], Value::String("cherry".into()));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_sort_with_comparator() {
    let result = eval_ts("[3, 1, 4, 1, 5].sort((a, b) => a - b)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Int(1));
            assert_eq!(arr[1], Value::Int(1));
            assert_eq!(arr[2], Value::Int(3));
            assert_eq!(arr[3], Value::Int(4));
            assert_eq!(arr[4], Value::Int(5));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_sort_descending() {
    let result = eval_ts("[3, 1, 4, 1, 5].sort((a, b) => b - a)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Int(5));
            assert_eq!(arr[1], Value::Int(4));
            assert_eq!(arr[2], Value::Int(3));
            assert_eq!(arr[3], Value::Int(1));
            assert_eq!(arr[4], Value::Int(1));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_flat_map() {
    let result = eval_ts("[1, 2, 3].flatMap((x) => [x, x * 2])").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 6);
            assert_eq!(arr[0], Value::Int(1));
            assert_eq!(arr[1], Value::Int(2));
            assert_eq!(arr[2], Value::Int(2));
            assert_eq!(arr[3], Value::Int(4));
            assert_eq!(arr[4], Value::Int(3));
            assert_eq!(arr[5], Value::Int(6));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_flat_map_non_array() {
    let result = eval_ts("[1, 2, 3].flatMap((x) => x * 2)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Int(2));
            assert_eq!(arr[1], Value::Int(4));
            assert_eq!(arr[2], Value::Int(6));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_map_with_closure() {
    let result = eval_ts(
        r#"
        const multiplier = 10;
        [1, 2, 3].map((x) => x * multiplier)
        "#,
    )
    .unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Int(10));
            assert_eq!(arr[1], Value::Int(20));
            assert_eq!(arr[2], Value::Int(30));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_chained_methods() {
    let result = eval_ts("[1, 2, 3, 4, 5].filter((x) => x % 2 === 0).map((x) => x * 10)").unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], Value::Int(20));
            assert_eq!(arr[1], Value::Int(40));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_array_every_empty() {
    // every on empty array returns true (vacuous truth)
    let result = eval_ts("[].every((x) => x > 0)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_array_some_empty() {
    // some on empty array returns false
    let result = eval_ts("[].some((x) => x > 0)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

// ── global builtin functions (regression: parseInt/parseFloat were missing) ──

#[test]
fn test_parse_int() {
    assert_eq!(eval_ts("parseInt('42px')").unwrap(), Value::Int(42));
    assert_eq!(eval_ts("parseInt('  -7')").unwrap(), Value::Int(-7));
    assert_eq!(eval_ts("parseInt('0xFF')").unwrap(), Value::Int(255));
    assert_eq!(eval_ts("parseInt('101', 2)").unwrap(), Value::Int(5));
    assert_eq!(eval_ts("parseInt('ff', 16)").unwrap(), Value::Int(255));
    assert!(matches!(eval_ts("parseInt('abc')").unwrap(), Value::Float(f) if f.is_nan()));
}

#[test]
fn test_parse_float() {
    assert_eq!(eval_ts("parseFloat('2.5xyz')").unwrap(), Value::Float(2.5));
    assert_eq!(
        eval_ts("parseFloat('  1e3')").unwrap(),
        Value::Float(1000.0)
    );
    assert!(matches!(eval_ts("parseFloat('nope')").unwrap(), Value::Float(f) if f.is_nan()));
}

#[test]
fn test_isnan_isfinite_number_string_boolean() {
    assert_eq!(eval_ts("isNaN(NaN)").unwrap(), Value::Bool(true));
    assert_eq!(eval_ts("isNaN(3)").unwrap(), Value::Bool(false));
    assert_eq!(eval_ts("isFinite(1/0)").unwrap(), Value::Bool(false));
    assert_eq!(eval_ts("isFinite(42)").unwrap(), Value::Bool(true));
    assert_eq!(eval_ts("Number('42')").unwrap(), Value::Int(42));
    assert_eq!(eval_ts("String(42)").unwrap(), Value::String("42".into()));
    assert_eq!(eval_ts("Boolean(0)").unwrap(), Value::Bool(false));
}

#[test]
fn test_global_builtin_shadowed_by_local() {
    // A local named `Number` must win over the builtin.
    let r = eval_ts("const Number = (x) => x + 1; Number(41)").unwrap();
    assert_eq!(r, Value::Int(42));
}
