// Should NOT trigger: expect in test code
#[cfg(test)]
mod tests {
    #[test]
    fn test_parsing() {
        let result = "42".parse::<i32>().expect("valid number");
        assert_eq!(result, 42);
    }
}
