#[derive(Debug, Clone)]
pub struct LocalIdentity {
    username: String,
}

impl LocalIdentity {
    pub fn new(username: impl Into<String>) -> Result<Self, String> {
        let username = username.into().trim().to_string();

        if username.is_empty() {
            return Err("Username cannot be empty.".into());
        }

        if username.len() > 32 {
            return Err("Username cannot be longer than 32 characters.".into());
        }

        Ok(Self { username })
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn set_username(&mut self, username: String) {
        self.username = username;
    }
}