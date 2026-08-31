use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use ed25519_dalek::{SigningKey, VerifyingKey};
use quinn::{ClientConfig, Endpoint, ServerConfig};
use rand::rngs::OsRng;
use rcgen::generate_simple_self_signed;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use rustls::pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime,
};
use rustls::{DigitallySignedStruct, SignatureScheme};

const YAP_VERSION: &str = "0.1.0";
const DEFAULT_PORT: u16 = 7331;

struct DangerousVerifier;

impl std::fmt::Debug for DangerousVerifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DangerousVerifier")
    }
}

impl ServerCertVerifier for DangerousVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PKCS1_SHA256,
        ]
    }
}

fn identity_path() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    home.join(".yap").join("identity.key")
}

fn load_or_generate_identity() -> Result<SigningKey, Box<dyn std::error::Error>> {
    let path = identity_path();

    if path.exists() {
        let bytes = std::fs::read(&path)?;

        if bytes.len() != 32 {
            return Err(format!(
                "YAP identity file is invalid: expected 32 bytes, found {}",
                bytes.len()
            )
            .into());
        }

        let secret: [u8; 32] = bytes.try_into().unwrap();

        return Ok(SigningKey::from_bytes(&secret));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let signing_key = SigningKey::generate(&mut OsRng);

    std::fs::write(&path, signing_key.to_bytes())?;

    Ok(signing_key)
}

fn identity_string(key: &SigningKey) -> String {
    let public_key: VerifyingKey = key.verifying_key();
    let encoded = hex_encode(public_key.as_bytes());

    format!("yap://{encoded}")
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }

    output
}

fn make_server_config() -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let certificate = generate_simple_self_signed(vec!["yap".to_string()])?;

    let certificate_der = certificate.cert.der().clone();

    let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        certificate.key_pair.serialize_der(),
    ));

    let mut server_config =
        ServerConfig::with_single_cert(vec![certificate_der], private_key)?;

    let transport_config = Arc::get_mut(&mut server_config.transport)
        .ok_or("failed to access QUIC transport configuration")?;

    transport_config.max_concurrent_bidi_streams(64u32.into());
    transport_config.max_concurrent_uni_streams(64u32.into());

    Ok(server_config)
}

fn make_client_config() -> Result<ClientConfig, Box<dyn std::error::Error>> {
    let crypto_provider = rustls::crypto::ring::default_provider();

    let tls_config = rustls::ClientConfig::builder_with_provider(crypto_provider.into())
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(DangerousVerifier))
        .with_no_client_auth();

    let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?;

    Ok(ClientConfig::new(Arc::new(quic_config)))
}

async fn listen() -> Result<(), Box<dyn std::error::Error>> {
    let server_config = make_server_config()?;

    let address: SocketAddr = format!("0.0.0.0:{DEFAULT_PORT}").parse()?;

    let endpoint = Endpoint::server(server_config, address)?;

    println!("YAP listener");
    println!("────────────");
    println!("Listening on {address}");
    println!("Protocol: YAP/1");
    println!("Transport: QUIC");
    println!();
    println!("Waiting for peers...");
    println!("Press Ctrl+C to stop.");

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            let connection = match connecting.await {
                Ok(connection) => connection,
                Err(error) => {
                    eprintln!("[!] Connection handshake failed: {error}");
                    return;
                }
            };

            println!(
                "[+] Peer connected from {}",
                connection.remote_address()
            );

            loop {
                match connection.accept_bi().await {
                    Ok((mut send, mut recv)) => {
                        let mut buffer = vec![0u8; 64 * 1024];

                        match recv.read(&mut buffer).await {
                            Ok(Some(length)) => {
                                let received = &buffer[..length];

                                println!(
                                    "[<] Received {} bytes: {}",
                                    length,
                                    String::from_utf8_lossy(received)
                                );

                                if let Err(error) = send.write_all(received).await {
                                    eprintln!("[!] Failed to echo message: {error}");
                                    break;
                                }

                                if let Err(error) = send.finish() {
                                    eprintln!("[!] Failed to finish stream: {error}");
                                    break;
                                }

                                println!("[>] Echoed message back to peer.");
                            }

                            Ok(None) => {
                                println!("[*] Peer closed the stream.");
                            }

                            Err(error) => {
                                eprintln!("[!] Stream read failed: {error}");
                                break;
                            }
                        }
                    }

                    Err(error) => {
                        eprintln!("[!] Failed to accept stream: {error}");
                        break;
                    }
                }
            }

            println!("[-] Peer disconnected.");
        });
    }

    Ok(())
}

async fn connect(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let socket: SocketAddr = address.parse()?;

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;

    endpoint.set_default_client_config(make_client_config()?);

    println!("Connecting to {socket}...");

    let connection = endpoint.connect(socket, "yap")?.await?;

    println!("Connected to {}", connection.remote_address());
    println!("QUIC handshake: OK");

    let (mut send, mut recv) = connection.open_bi().await?;

    let message = b"YAP/1 HELLO";

    println!("[>] Sending {} bytes...", message.len());

    send.write_all(message).await?;
    send.finish()?;

    let mut buffer = vec![0u8; 64 * 1024];

    match recv.read(&mut buffer).await? {
        Some(length) => {
            let response = &buffer[..length];

            println!(
                "[<] Received {} bytes: {}",
                length,
                String::from_utf8_lossy(response)
            );

            if response == message {
                println!();
                println!("YAP/1 QUIC test: SUCCESS");
                println!("The peer received and echoed our message.");
            } else {
                println!();
                println!("YAP/1 QUIC test: RESPONSE RECEIVED");
            }
        }

        None => {
            println!("Peer closed the stream without sending a response.");
        }
    }

    endpoint.wait_idle().await;

    Ok(())
}

fn print_help() {
    println!(
        r#"
YAP — peer-to-peer communication

GENERAL
  help                    Show this help
  clear                   Clear the terminal
  quit                    Exit YAP
  version                 Show YAP version
  status                  Show YAP status

IDENTITY
  id                      Show your YAP identity
  id generate             Generate a new identity
  id export               Export your public identity
  id fingerprint          Show your identity fingerprint

PEERS
  connect <address>       Connect to a peer
  disconnect <peer>       Disconnect from a peer
  peers                   List connected peers
  peer <peer>             Show peer information
  ping <peer>             Ping a peer

MESSAGING
  say <message>           Send a message
  msg <peer> <message>    Send a private message
  history                 Show message history
  history clear           Clear message history

NETWORK
  listen <address>        Listen for incoming connections
  connections             Show active connections
  network                 Show network information

ROOMS
  room create <name>      Create a room
  room join <name>        Join a room
  room leave <name>       Leave a room
  rooms                   List rooms
  room <name>             Show room information

FILES
  send <peer> <file>      Send a file
  downloads               Show active downloads
  uploads                 Show active uploads

SETTINGS
  set <option> <value>    Change a setting
  settings                Show current settings

DEBUG
  debug                   Show debug information
  protocol                Show YAP protocol information

YAP/1
  Binary protocol running over QUIC.
"#
    );
}

fn print_id(identity: &SigningKey) {
    let public_key = identity.verifying_key();

    println!("YAP identity");
    println!("────────────");
    println!("Address:     {}", identity_string(identity));
    println!("Public key:  {}", hex_encode(public_key.as_bytes()));
    println!("Storage:     {}", identity_path().display());
}

fn print_status(identity: &SigningKey) {
    println!("YAP status");
    println!("──────────");
    println!("Version:     {YAP_VERSION}");
    println!("Protocol:    YAP/1");
    println!("Transport:   QUIC");
    println!("Identity:    {}", identity_string(identity));
    println!("Port:        {DEFAULT_PORT}");
}

#[tokio::main]
async fn main() {
    let identity = match load_or_generate_identity() {
        Ok(identity) => identity,
        Err(error) => {
            eprintln!("Failed to load YAP identity: {error}");
            return;
        }
    };

    println!("YAP v{YAP_VERSION}");
    println!("Peer-to-peer communication");
    println!("Identity: {}", identity_string(&identity));
    println!("Type 'help' for commands.");
    println!();

    let mut editor = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(error) => {
            eprintln!("Failed to start YAP: {error}");
            return;
        }
    };

    loop {
        match editor.readline("yap> ") {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                let _ = editor.add_history_entry(input);

                match input {
                    "help" => print_help(),

                    "id" => {
                        print_id(&identity);
                    }

                    "id generate" => {
                        println!(
                            "Identity already exists at {}.",
                            identity_path().display()
                        );
                        println!(
                            "Delete that file if you want to generate a new identity."
                        );
                    }

                    "id export" => {
                        println!("{}", identity_string(&identity));
                    }

                    "id fingerprint" => {
                        println!(
                            "{}",
                            hex_encode(identity.verifying_key().as_bytes())
                        );
                    }

                    "peers" => {
                        println!("No persistent peer connections yet.");
                    }

                    "clear" => {
                        print!("\x1B[2J\x1B[1;1H");
                    }

                    "version" => {
                        println!("YAP v{YAP_VERSION}");
                    }

                    "status" => {
                        print_status(&identity);
                    }

                    "listen" => {
                        if let Err(error) = listen().await {
                            eprintln!("Listener error: {error}");
                        }

                        break;
                    }

                    command if command.starts_with("listen ") => {
                        println!(
                            "Custom listen addresses aren't wired in yet."
                        );
                        println!(
                            "Use 'listen' for 0.0.0.0:{DEFAULT_PORT}."
                        );
                    }

                    command if command.starts_with("connect ") => {
                        let address = command.strip_prefix("connect ").unwrap();

                        if let Err(error) = connect(address).await {
                            eprintln!("Connection failed: {error}");
                        }
                    }

                    command if command.starts_with("say ") => {
                        let message = command.strip_prefix("say ").unwrap();

                        println!("[you] {message}");
                        println!(
                            "Messaging transport isn't wired into the shell yet."
                        );
                    }

                    command if command.starts_with("disconnect ") => {
                        println!("Peer connections aren't persistent yet.");
                    }

                    command if command.starts_with("peer ") => {
                        println!("Peer inspection isn't implemented yet.");
                    }

                    command if command.starts_with("ping ") => {
                        println!("Ping isn't implemented yet.");
                    }

                    command if command.starts_with("msg ") => {
                        println!("Private messaging isn't implemented yet.");
                    }

                    command if command.starts_with("history") => {
                        println!("Message history isn't implemented yet.");
                    }

                    command if command.starts_with("room ")
                        || command == "rooms" =>
                    {
                        println!("Rooms aren't implemented yet.");
                    }

                    command if command.starts_with("send ") => {
                        println!("File transfer isn't implemented yet.");
                    }

                    "downloads" | "uploads" => {
                        println!("File transfer isn't implemented yet.");
                    }

                    command if command.starts_with("set ")
                        || command == "settings" =>
                    {
                        println!("Settings aren't implemented yet.");
                    }

                    "connections" | "network" => {
                        println!("Network information isn't implemented yet.");
                    }

                    "debug" | "protocol" => {
                        println!("Debug information isn't implemented yet.");
                    }

                    "quit" | "exit" => {
                        println!("Goodbye.");
                        break;
                    }

                    _ => {
                        println!("Unknown command. Type 'help'.");
                    }
                }
            }

            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }

            Err(ReadlineError::Eof) => {
                println!("^D");
                break;
            }

            Err(error) => {
                eprintln!("YAP error: {error}");
                break;
            }
        }
    }
}