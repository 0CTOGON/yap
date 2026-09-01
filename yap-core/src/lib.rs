use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    sync::Arc,
};

use quinn::{
    ClientConfig,
    Endpoint,
    ServerConfig,
};

use rustls::{
    client::danger::{
        HandshakeSignatureValid,
        ServerCertVerified,
        ServerCertVerifier,
    },
    pki_types::{
        CertificateDer,
        PrivateKeyDer,
        ServerName,
        UnixTime,
    },
    DigitallySignedStruct,
    SignatureScheme,
};

use tokio::sync::{
    mpsc::UnboundedSender,
    Mutex,
};

use yap_protocol::Packet;

mod identity;

pub use identity::LocalIdentity;

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

struct PeerConnection {
    connection: quinn::Connection,
}

pub struct Network {
    identity: Arc<Mutex<LocalIdentity>>,
    endpoint: Endpoint,
    peers: Arc<Mutex<HashMap<String, PeerConnection>>>,
    events_tx: UnboundedSender<IncomingMessage>,
}

impl Network {
    pub async fn bind(
        identity: LocalIdentity,
        port: u16,
        events_tx: UnboundedSender<IncomingMessage>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let server_config = make_server_config()?;

        let endpoint = Endpoint::server(
            server_config,
            SocketAddr::from(([0, 0, 0, 0], port)),
        )?;

        let network = Self {
            identity: Arc::new(Mutex::new(identity)),
            endpoint,
            peers: Arc::new(Mutex::new(HashMap::new())),
            events_tx,
        };

        network.start_accept_loop();

        let mut network = network;
        network.configure_client()?;

        Ok(network)
    }

    pub async fn username(&self) -> String {
        self.identity
            .lock()
            .await
            .username()
            .to_string()
    }

    pub fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        self.endpoint.local_addr()
    }

    pub async fn set_username(
        &self,
        username: &str,
    ) -> Result<(), String> {
        let username = username.trim();

        if username.is_empty() {
            return Err("Username cannot be empty.".into());
        }

        if username.len() > 32 {
            return Err("Username cannot be longer than 32 characters.".into());
        }

        self.identity
            .lock()
            .await
            .set_username(username.to_string());

        Ok(())
    }

    pub async fn connect(
        &self,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let address: SocketAddr = address.parse()?;

        let connecting = self.endpoint.connect(address, "localhost")?;

        let connection = connecting.await?;

        let username = self.username().await;

        send_packet(
            &connection,
            &Packet::Hello {
                username,
            },
        )
        .await?;

        let peers = Arc::clone(&self.peers);
        let events = self.events_tx.clone();

        tokio::spawn(async move {
            if let Err(error) =
                handle_connection(
                    connection,
                    peers,
                    events,
                )
                .await
            {
                eprintln!("Connection error: {error}");
            }
        });

        Ok(address.to_string())
    }

    pub async fn disconnect(
        &self,
        username: &str,
    ) -> Result<(), String> {
        let mut peers = self.peers.lock().await;

        if let Some(peer) = peers.remove(username) {
            peer.connection
                .close(0u32.into(), b"goodbye");

            let _ = self.events_tx.send(
                IncomingMessage::PeerDisconnected {
                    username: username.to_string(),
                },
            );

            Ok(())
        } else {
            Err(format!(
                "Peer '{username}' is not connected."
            ))
        }
    }

    pub async fn peers(&self) -> Vec<String> {
        let peers = self.peers.lock().await;

        let mut names: Vec<String> =
            peers.keys().cloned().collect();

        names.sort();

        names
    }

    pub async fn broadcast(
        &self,
        message: &str,
    ) -> Result<(), String> {
        let username = self.username().await;

        let packet = Packet::Chat {
            from: username,
            message: message.to_string(),
        };

        let peers = self.peers.lock().await;

        for peer in peers.values() {
            send_packet(
                &peer.connection,
                &packet,
            )
            .await
            .map_err(|error| error.to_string())?;
        }

        Ok(())
    }

    pub async fn send_direct(
        &self,
        target: &str,
        message: &str,
    ) -> Result<(), String> {
        let username = self.username().await;

        let peers = self.peers.lock().await;

        let peer = peers
            .get(target)
            .ok_or_else(|| {
                format!(
                    "Peer '{target}' is not connected."
                )
            })?;

        let packet = Packet::Direct {
            from: username,
            to: target.to_string(),
            message: message.to_string(),
        };

        send_packet(
            &peer.connection,
            &packet,
        )
        .await
        .map_err(|error| error.to_string())
    }

    fn start_accept_loop(&self) {
        let endpoint = self.endpoint.clone();
        let peers = Arc::clone(&self.peers);
        let events = self.events_tx.clone();

        tokio::spawn(async move {
            while let Some(incoming) =
                endpoint.accept().await
            {
                let peers = Arc::clone(&peers);
                let events = events.clone();

                tokio::spawn(async move {
                    match incoming.await {
                        Ok(connection) => {
                            if let Err(error) =
                                handle_connection(
                                    connection,
                                    peers,
                                    events,
                                )
                                .await
                            {
                                eprintln!(
                                    "Incoming connection error: {error}"
                                );
                            }
                        }

                        Err(error) => {
                            eprintln!(
                                "Failed to accept connection: {error}"
                            );
                        }
                    }
                });
            }
        });
    }
}

async fn send_packet(
    connection: &quinn::Connection,
    packet: &Packet,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = serde_json::to_vec(packet)?;

    let mut stream =
        connection.open_uni().await?;

    tokio::io::AsyncWriteExt::write_all(
        &mut stream,
        &data,
    )
    .await?;

    stream.finish()?;

    Ok(())
}

async fn handle_connection(
    connection: quinn::Connection,
    peers: Arc<Mutex<HashMap<String, PeerConnection>>>,
    events: UnboundedSender<IncomingMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let mut stream =
            connection.accept_uni().await?;

        let mut data = Vec::new();

        tokio::io::AsyncReadExt::read_to_end(
            &mut stream,
            &mut data,
        )
        .await?;

        let packet: Packet =
            serde_json::from_slice(&data)?;

        match packet {
            Packet::Hello { username } => {
                peers.lock().await.insert(
                    username.clone(),
                    PeerConnection {
                        connection: connection.clone(),
                    },
                );

                let _ = events.send(
                    IncomingMessage::PeerConnected {
                        username,
                        address: connection.remote_address(),
                    },
                );
            }

            Packet::Chat {
                from,
                message,
            } => {
                let _ = events.send(
                    IncomingMessage::Chat {
                        from,
                        message,
                    },
                );
            }

            Packet::Direct {
                from,
                to,
                message,
            } => {
                let local_username = {
                    let identity =
                        peers.lock().await;

                    drop(identity);

                    None::<String>
                };

                let _ = local_username;

                let _ = events.send(
                    IncomingMessage::Direct {
                        from,
                        to,
                        message,
                    },
                );
            }
        }
    }
}

fn make_server_config(
) -> Result<
    ServerConfig,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let certified =
        rcgen::generate_simple_self_signed(
            vec!["localhost".to_string()],
        )?;

    let cert_der =
        certified.cert.der().to_vec();

    let key_der =
        certified.key_pair.serialize_der();

    let cert_chain = vec![
        CertificateDer::from(cert_der),
    ];

    let private_key =
        PrivateKeyDer::try_from(key_der)?;

    let mut config =
        ServerConfig::with_single_cert(
            cert_chain,
            private_key,
        )?;

    let transport =
        Arc::get_mut(&mut config.transport)
            .expect(
                "transport config should be uniquely owned",
            );

    transport
        .max_concurrent_uni_streams(128_u32.into());

    Ok(config)
}

#[derive(Debug)]
struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<
        ServerCertVerified,
        rustls::Error,
    > {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<
        HandshakeSignatureValid,
        rustls::Error,
    > {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider()
                .signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<
        HandshakeSignatureValid,
        rustls::Error,
    > {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider()
                .signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(
        &self,
    ) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn make_client_config() -> Result<
    ClientConfig,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(
            Arc::new(SkipServerVerification),
        )
        .with_no_client_auth();

    let crypto =
        quinn::crypto::rustls::QuicClientConfig::try_from(
            crypto,
        )?;

    Ok(ClientConfig::new(
        Arc::new(crypto),
    ))
}

impl Network {
    #[allow(dead_code)]
    fn configure_client(
        &mut self,
    ) -> Result<
        (),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let config = make_client_config()?;

        self.endpoint
            .set_default_client_config(config);

        Ok(())
    }
}