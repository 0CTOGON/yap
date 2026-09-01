use crate::error::ProtocolError;

const MAX_TEXT_LENGTH: usize = u32::MAX as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Identity = 1,
    Direct = 2,
    Chat = 3,
}

impl PacketType {
    fn from_u8(value: u8) -> Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::Identity),
            2 => Ok(Self::Direct),
            3 => Ok(Self::Chat),
            other => Err(ProtocolError::UnknownPacketType(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Identity {
        username: String,
    },

    Direct {
        from: String,
        to: String,
        message: String,
    },

    Chat {
        from: String,
        message: String,
    },
}

impl Packet {
    pub fn identity(username: impl Into<String>) -> Result<Self, ProtocolError> {
        let username = username.into();

        validate_text(&username)?;

        Ok(Self::Identity { username })
    }

    pub fn direct(
        from: impl Into<String>,
        to: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<Self, ProtocolError> {
        let from = from.into();
        let to = to.into();
        let message = message.into();

        validate_text(&from)?;
        validate_text(&to)?;
        validate_text(&message)?;

        Ok(Self::Direct {
            from,
            to,
            message,
        })
    }

    pub fn chat(
        from: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<Self, ProtocolError> {
        let from = from.into();
        let message = message.into();

        validate_text(&from)?;
        validate_text(&message)?;

        Ok(Self::Chat { from, message })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut output = Vec::new();

        match self {
            Self::Identity { username } => {
                output.push(PacketType::Identity as u8);
                write_string(&mut output, username)?;
            }

            Self::Direct {
                from,
                to,
                message,
            } => {
                output.push(PacketType::Direct as u8);

                write_string(&mut output, from)?;
                write_string(&mut output, to)?;
                write_string(&mut output, message)?;
            }

            Self::Chat { from, message } => {
                output.push(PacketType::Chat as u8);

                write_string(&mut output, from)?;
                write_string(&mut output, message)?;
            }
        }

        Ok(output)
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.is_empty() {
            return Err(ProtocolError::TooShort);
        }

        let packet_type = PacketType::from_u8(data[0])?;
        let mut cursor = 1;

        match packet_type {
            PacketType::Identity => {
                let username = read_string(data, &mut cursor)?;

                Ok(Self::Identity { username })
            }

            PacketType::Direct => {
                let from = read_string(data, &mut cursor)?;
                let to = read_string(data, &mut cursor)?;
                let message = read_string(data, &mut cursor)?;

                Ok(Self::Direct {
                    from,
                    to,
                    message,
                })
            }

            PacketType::Chat => {
                let from = read_string(data, &mut cursor)?;
                let message = read_string(data, &mut cursor)?;

                Ok(Self::Chat { from, message })
            }
        }
    }
}

fn validate_text(value: &str) -> Result<(), ProtocolError> {
    if value.len() > MAX_TEXT_LENGTH {
        return Err(ProtocolError::FieldTooLarge);
    }

    Ok(())
}

fn write_string(output: &mut Vec<u8>, value: &str) -> Result<(), ProtocolError> {
    validate_text(value)?;

    let bytes = value.as_bytes();
    let length = bytes.len() as u32;

    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);

    Ok(())
}

fn read_string(data: &[u8], cursor: &mut usize) -> Result<String, ProtocolError> {
    if data.len() < *cursor + 4 {
        return Err(ProtocolError::TooShort);
    }

    let length = u32::from_be_bytes([
        data[*cursor],
        data[*cursor + 1],
        data[*cursor + 2],
        data[*cursor + 3],
    ]) as usize;

    *cursor += 4;

    if data.len() < *cursor + length {
        return Err(ProtocolError::TooShort);
    }

    let value = std::str::from_utf8(&data[*cursor..*cursor + length])
        .map_err(|_| ProtocolError::InvalidUtf8)?
        .to_string();

    *cursor += length;

    Ok(value)
}