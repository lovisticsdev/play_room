use crate::event::ServerEvent;
use crate::request::ClientRequest;
use crate::response::ServerResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientEnvelope {
    pub request_id: u64,
    pub request: ClientRequest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ServerMessage {
    Response {
        request_id: u64,
        result: ServerResult,
    },
    Event {
        event: ServerEvent,
    },
}
