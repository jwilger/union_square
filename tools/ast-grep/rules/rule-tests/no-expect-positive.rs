// Should trigger: expect in production code
fn parse(input: &str) -> i32 {
    input.parse::<i32>().expect("must be a number")
}
