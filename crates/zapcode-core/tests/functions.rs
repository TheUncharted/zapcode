use zapcode_core::vm::eval_ts;
use zapcode_core::Value;

#[test]
fn test_function_declaration() {
    let result = eval_ts("function add(a, b) { return a + b; } add(2, 3)").unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_arrow_function() {
    let result = eval_ts("const double = (x) => x * 2; double(21)").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_arrow_function_block() {
    let result =
        eval_ts("const greet = (name) => { return \"hello \" + name; }; greet(\"world\")").unwrap();
    assert_eq!(result, Value::String("hello world".into()));
}

#[test]
fn test_recursive_function() {
    let result = eval_ts(
        "function factorial(n) { if (n <= 1) { return 1; } return n * factorial(n - 1); } factorial(5)",
    )
    .unwrap();
    assert_eq!(result, Value::Int(120));
}

#[test]
fn test_function_no_return() {
    let result = eval_ts("function noop() {} noop()").unwrap();
    assert_eq!(result, Value::Undefined);
}

#[test]
fn test_function_expression() {
    let result = eval_ts("const mul = function(a, b) { return a * b; }; mul(6, 7)").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_higher_order_function() {
    let result =
        eval_ts("function apply(f, x) { return f(x); } const inc = (x) => x + 1; apply(inc, 41)")
            .unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_rest_params() {
    let result = eval_ts(
        "function sum(...nums) { let total = 0; for (let i = 0; i < nums.length; i++) { total += nums[i]; } return total; } sum(1, 2, 3, 4, 5)",
    )
    .unwrap();
    // Rest params create an array — need array indexing to work
    assert_eq!(result, Value::Int(15));
}

// ── destructuring parameters (regression: bound whole arg, not elements) ─────

#[test]
fn test_array_destructure_param() {
    let r = eval_ts("const f = ([a, b]) => a + b; f([10, 20])").unwrap();
    assert_eq!(r, Value::Int(30));
}

#[test]
fn test_object_destructure_param() {
    let r = eval_ts("const f = ({ x, y }) => x * y; f({ x: 6, y: 7 })").unwrap();
    assert_eq!(r, Value::Int(42));
}

#[test]
fn test_entries_map_destructure() {
    let r =
        eval_ts("Object.entries({ a: 1, b: 2 }).map(([k, v]) => `${k}${v}`).join(',')").unwrap();
    assert_eq!(r, Value::String("a1,b2".into()));
}
