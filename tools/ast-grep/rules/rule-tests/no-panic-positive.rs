// Should trigger: panic in production code
fn validate(value: i32) {
    if value < 0 {
        panic!("value must be positive");
    }
}
