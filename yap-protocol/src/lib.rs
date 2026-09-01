use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Packet {
    Hello {
        username: String,
    },

    Chat {
        from: String,
        message: String,
    },

    Direct {
        from: String,
        to: String,
        message: String,
    },
}