//! JSON-lines protocol for Play Room.

pub mod codec;
pub mod error;
pub mod event;
pub mod manifest;
pub mod message;
pub mod request;
pub mod response;
pub mod schema;
pub mod version;

pub use codec::{decode_client, decode_server, encode_client, encode_server};
pub use error::ProtocolError;
pub use event::ServerEvent;
pub use manifest::{
    protocol_manifest, typescript_constants_module, typescript_types_module, ProtocolManifest,
};
pub use message::{ClientEnvelope, ServerMessage};
pub use request::ClientRequest;
pub use response::{ErrorCode, ServerResult};
pub use schema::{client_envelope_schema, server_message_schema, typescript_schema_module};
pub use version::PROTOCOL_VERSION;
