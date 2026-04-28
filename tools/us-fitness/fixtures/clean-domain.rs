pub struct AccountId(String);

impl AccountId {
    pub fn new(value: String) -> Result<Self, String> {
        let value = value.trim().to_string();
        if value.is_empty() {
            Err("account id must not be empty".to_string())
        } else {
            Ok(Self(value))
        }
    }
}
