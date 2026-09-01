#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub username: String,
    pub address: String,
}

impl PeerInfo {
    pub fn new(username: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            address: address.into(),
        }
    }
}