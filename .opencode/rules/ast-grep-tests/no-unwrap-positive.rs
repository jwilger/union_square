// Should trigger: unwrap in production code
fn process(value: Option<i32>) -> i32 {
    value.unwrap()
}
