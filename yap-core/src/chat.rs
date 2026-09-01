use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Peer {
    pub username: String,
    pub address: String,
}

pub struct Chat {
    peers: HashMap<String, Peer>,
}

impl Chat {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, username: String, address: String) {
        self.peers.insert(
            username.clone(),
            Peer {
                username,
                address,
            },
        );
    }

    pub fn remove_peer(&mut self, username: &str) {
        self.peers.remove(username);
    }

    pub fn peers(&self) -> impl Iterator<Item = &Peer> {
        self.peers.values()
    }
}