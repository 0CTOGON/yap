use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("QUIC error: {0}")]
    Quinn(#[from] quinn::ConnectionError),

    #[error("QUIC write error: {0}")]
    Write(#[from] quinn::WriteError),

    #[error("QUIC read error: {0}")]
    Read(#[from] quinn::ReadToEndError),

    #[error("protocol error: {0}")]
    Protocol(#[from] yap_protocol::ProtocolError),

    #[error("TLS configuration error: {0}")]
    Tls(String),

    #[error("invalid address: {0}")]
    InvalidAddress(#[from] std::net::AddrParseError),

    #[error("peer already exists: {0}")]
    PeerAlreadyExists(String),

    #[error("peer not found: {0}")]
    PeerNotFound(String),

    #[error("peer connection closed")]
    ConnectionClosed,

    #[error("invalid username: {0}")]
    InvalidUsername(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}