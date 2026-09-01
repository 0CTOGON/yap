use crate::error::ProtocolError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub username: String,
}

impl Identity {
    pub fn new(username: impl Into<String>) -> Result<Self, ProtocolError> {
        let username = username.into();

        if username.is_empty() || username.len() > u16::MAX as usize {
            return Err(ProtocolError::FieldTooLarge);
        }

        Ok(Self { username })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        let name = self.username.as_bytes();

        if name.len() > u16::MAX as usize {
            return Err(ProtocolError::FieldTooLarge);
        }

        let mut output = Vec::with_capacity(2 + name.len());

        output.extend_from_slice(&(name.len() as u16).to_be_bytes());
        output.extend_from_slice(name);

        Ok(output)
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 2 {
            return Err(ProtocolError::TooShort);
        }

        let length = u16::from_be_bytes([data[0], data[1]]) as usize;

        if data.len() < 2 + length {
            return Err(ProtocolError::TooShort);
        }

        let username = std::str::from_utf8(&data[2..2 + length])
            .map_err(|_| ProtocolError::InvalidUtf8)?
            .to_string();

        Self::new(username)
    }
}