use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind")]
pub enum ClientToServer {
    Register { nick: String, client_id: Uuid },
    CreateGroup { group: String },
    Invite { group: String, nick: String },
    JoinGroup { group: String, invite_code: String },
    SendMessage { group: String, text: String, nick: String },
    SendPvtMessage { to: String, text: String },
    GlobalMessage { text: String },
    ListGroups,
    ListUsers,
    Logout,
    Ping,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind")]
pub enum ServerToClient {
    Registered { ok: bool, reason: Option<String> },
    InviteCode { group: String, code: String, client_id: String },
    InviteCodeForMe { group: String, code: String },
    ListUsers {users : Vec<String>},
    Joined { group: String },
    Message { group: String, from: String, text: String },
    GlobalMessage { from: String, text: String },
    SendPvtMessage { from: String, text: String },
    Groups { groups: Vec<String> },
    Error { reason: String },
    Pong,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("io error: {0}")]
    Io(String),
    #[error("json error: {0}")]
    Json(String),
    #[error("protocol: {0}")]
    Proto(String),
}

pub type Result<T> = std::result::Result<T, ProtocolError>;
