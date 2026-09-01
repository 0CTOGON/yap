use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use quinn::{Endpoint, Incoming, RecvStream, SendStream};
use tokio::sync::mpsc;

use yap_protocol::Packet;

use crate::error::CoreError;
use crate::identity::{make_client_config, make_server_config, LocalIdentity};
use crate::peer::Peer;
use crate::peer_manager::PeerManager;

#[derive(Debug, Clone)]
pub enum IncomingMessage {
    Direct {
        from: String,
        to: String,
        message: String,
    },

    Chat {
        from: String,
        message: String,
    },

    PeerConnected {
        username: String,
        address: SocketAddr,
    },

    PeerDisconnected {
        username: String,
    },
}

#[derive(Clone)]
pub struct Network {
    endpoint: Endpoint,
    identity: Arc<tokio::sync::RwLock<LocalIdentity>>,
    peers: PeerManager,
    events: mpsc::UnboundedSender<IncomingMessage>,
}

impl Network {
    pub async fn bind(
        identity: LocalIdentity,
        port: u16,
        events: mpsc::UnboundedSender<IncomingMessage>,
    ) -> Result<Self, CoreError> {
        let server_config = make_server_config()?;

        let bind_addr =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);

        let endpoint = Endpoint::server(server_config, bind_addr)?;

        let network = Self {
            endpoint,
            identity: Arc::new(tokio::sync::RwLock::new(identity)),
            peers: PeerManager::new(),
            events,
        };

        network.spawn_accept_loop();

        Ok(network)
    }

    pub fn local_addr(&self) -> Result<SocketAddr, CoreError> {
        Ok(self.endpoint.local_addr()?)
    }

    pub async fn username(&self) -> String {
        self.identity.read().await.username().to_string()
    }

    pub async fn set_username(
        &self,
        username: impl Into<String>,
    ) -> Result<(), CoreError> {
        self.identity.write().await.set_username(username)
    }

    pub async fn connect(
        &self,
        address: SocketAddr,
    ) -> Result<String, CoreError> {
        let client_config = make_client_config()?;

        let endpoint = &self.endpoint;

        endpoint.set_default_client_config(client_config);

        let connecting = endpoint
            .connect(address, "yap")
            .map_err(|error| CoreError::Tls(error.to_string()))?;

        let connection = connecting.await?;

        let peer_address = connection.remote_address();

        let username = self.username().await;

        send_packet(
            &connection,
            Packet::identity(username).map_err(CoreError::Protocol)?,
        )
        .await?;

        let remote_username =
            receive_identity(&connection).await?;

        let peer = Peer::new(
            remote_username.clone(),
            peer_address,
            connection.clone(),
        );

        self.peers.add(peer).await?;

        self.spawn_receive_loop(
            remote_username.clone(),
            connection,
        );

        let _ = self.events.send(IncomingMessage::PeerConnected {
            username: remote_username.clone(),
            address: peer_address,
        });

        Ok(remote_username)
    }

    pub async fn disconnect(
        &self,
        username: &str,
    ) -> Result<(), CoreError> {
        let peer = self
            .peers
            .remove(username)
            .await
            .ok_or_else(|| CoreError::PeerNotFound(username.to_string()))?;

        peer.connection.close(0u32.into(), b"disconnect");

        let _ = self.events.send(
            IncomingMessage::PeerDisconnected {
                username: username.to_string(),
            },
        );

        Ok(())
    }

    pub async fn send_direct(
        &self,
        username: &str,
        message: &str,
    ) -> Result<(), CoreError> {
        let peer = self
            .peers
            .get(username)
            .await
            .ok_or_else(|| CoreError::PeerNotFound(username.to_string()))?;

        let from = self.username().await;

        let packet =
            Packet::direct(from, username, message)?;

        send_packet(&peer.connection, packet).await
    }

    pub async fn broadcast(
        &self,
        message: &str,
    ) -> Result<(), CoreError> {
        let peers = self.peers.all().await;
        let from = self.username().await;

        let packet = Packet::chat(from, message)?;

        for peer in peers {
            if let Err(error) =
                send_packet(&peer.connection, packet.clone()).await
            {
                eprintln!(
                    "Failed to send to {}: {}",
                    peer.username, error
                );
            }
        }

        Ok(())
    }

    pub async fn peer_names(&self) -> Vec<String> {
        self.peers.names().await
    }

    fn spawn_accept_loop(&self) {
        let network = self.clone();

        tokio::spawn(async move {
            loop {
                let incoming = match network.endpoint.accept().await {
                    Some(incoming) => incoming,
                    None => break,
                };

                let network = network.clone();

                tokio::spawn(async move {
                    if let Err(error) =
                        network.handle_incoming(incoming).await
                    {
                        eprintln!(
                            "Incoming connection failed: {}",
                            error
                        );
                    }
                });
            }
        });
    }

    async fn handle_incoming(
        &self,
        incoming: Incoming,
    ) -> Result<(), CoreError> {
        let connection = incoming.await?;
        let address = connection.remote_address();

        let remote_username =
            receive_identity(&connection).await?;

        let local_username = self.username().await;

        send_packet(
            &connection,
            Packet::identity(local_username)?,
        )
        .await?;

        let peer = Peer::new(
            remote_username.clone(),
            address,
            connection.clone(),
        );

        self.peers.add(peer).await?;

        self.spawn_receive_loop(
            remote_username.clone(),
            connection,
        );

        let _ = self.events.send(IncomingMessage::PeerConnected {
            username: remote_username,
            address,
        });

        Ok(())
    }

    fn spawn_receive_loop(
        &self,
        username: String,
        connection: quinn::Connection,
    ) {
        let peers = self.peers.clone();
        let events = self.events.clone();

        tokio::spawn(async move {
            loop {
                let stream = match connection.accept_uni().await {
                    Ok(stream) => stream,
                    Err(_) => break,
                };

                match read_packet(stream).await {
                    Ok(Packet::Direct {
                        from,
                        to,
                        message,
                    }) => {
                        let _ = events.send(
                            IncomingMessage::Direct {
                                from,
                                to,
                                message,
                            },
                        );
                    }

                    Ok(Packet::Chat { from, message }) => {
                        let _ = events.send(
                            IncomingMessage::Chat {
                                from,
                                message,
                            },
                        );
                    }

                    Ok(Packet::Identity { .. }) => {
                        // Identity packets are only valid during connection setup.
                    }

                    Err(error) => {
                        eprintln!(
                            "Packet from {} failed: {}",
                            username, error
                        );
                    }
                }
            }

            peers.remove(&username).await;

            let _ = events.send(
                IncomingMessage::PeerDisconnected {
                    username,
                },
            );
        });
    }
}

async fn send_packet(
    connection: &quinn::Connection,
    packet: Packet,
) -> Result<(), CoreError> {
    let data = packet.encode()?;

    let mut stream = connection.open_uni().await?;

    let length = data.len() as u32;

    stream.write_all(&length.to_be_bytes()).await?;
    stream.write_all(&data).await?;
    stream.finish()?;

    Ok(())
}

async fn read_packet(
    mut stream: RecvStream,
) -> Result<Packet, CoreError> {
    let length = stream.read_u32().await?;

    let data = stream.read_to_end(length as usize).await?;

    Ok(Packet::decode(&data)?)
}

async fn receive_identity(
    connection: &quinn::Connection,
) -> Result<String, CoreError> {
    let mut stream = connection.accept_uni().await?;

    let length = stream.read_u32().await?;

    let data = stream.read_to_end(length as usize).await?;

    match Packet::decode(&data)? {
        Packet::Identity { username } => Ok(username),

        _ => Err(CoreError::ConnectionClosed),
    }
}