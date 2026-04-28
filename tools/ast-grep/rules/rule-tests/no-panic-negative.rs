// Should NOT trigger: panic in test code
#[cfg(test)]
mod tests {
    #[test]
    #[should_panic]
    fn test_panics() {
        panic!("expected panic");
    }
}
