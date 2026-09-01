use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::CoreError;
use crate::peer::Peer;

#[derive(Clone, Default)]
pub struct PeerManager {
    peers: Arc<RwLock<HashMap<String, Peer>>>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn add(&self, peer: Peer) -> Result<(), CoreError> {
        let mut peers = self.peers.write().await;

        if peers.contains_key(&peer.username) {
            return Err(CoreError::PeerAlreadyExists(peer.username));
        }

        peers.insert(peer.username.clone(), peer);

        Ok(())
    }

    pub async fn remove(&self, username: &str) -> Option<Peer> {
        self.peers.write().await.remove(username)
    }

    pub async fn get(&self, username: &str) -> Option<Peer> {
        self.peers.read().await.get(username).cloned()
    }

    pub async fn names(&self) -> Vec<String> {
        let peers = self.peers.read().await;

        let mut names: Vec<String> = peers.keys().cloned().collect();
        names.sort();

        names
    }

    pub async fn all(&self) -> Vec<Peer> {
        self.peers.read().await.values().cloned().collect()
    }
}