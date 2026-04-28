// Should NOT trigger: unwrap in test code
#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        let value = Some(42);
        assert_eq!(value.unwrap(), 42);
    }
}
