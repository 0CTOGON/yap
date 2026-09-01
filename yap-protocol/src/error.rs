use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("packet is too short")]
    TooShort,

    #[error("unknown packet type: {0}")]
    UnknownPacketType(u8),

    #[error("invalid UTF-8")]
    InvalidUtf8,

    #[error("field is too large")]
    FieldTooLarge,

    #[error("invalid packet")]
    InvalidPacket,
}