//! JSON-lines protocol for Play Room.

pub mod codec;
pub mod error;
pub mod event;
pub mod message;
pub mod request;
pub mod response;
pub mod version;

pub use codec::{decode_client, decode_server, encode_client, encode_server};
pub use error::ProtocolError;
pub use event::ServerEvent;
pub use message::{ClientEnvelope, ServerMessage};
pub use request::ClientRequest;
pub use response::ServerResult;
pub use version::PROTOCOL_VERSION;
