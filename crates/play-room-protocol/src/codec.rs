use crate::error::ProtocolError;
use crate::message::{ClientEnvelope, ServerMessage};

pub fn encode_client(message: &ClientEnvelope) -> Result<String, ProtocolError> {
    Ok(format!("{}\n", serde_json::to_string(message)?))
}

pub fn decode_client(line: &str) -> Result<ClientEnvelope, ProtocolError> {
    if line.trim().is_empty() {
        return Err(ProtocolError::EmptyLine);
    }
    Ok(serde_json::from_str(line.trim_end())?)
}

pub fn encode_server(message: &ServerMessage) -> Result<String, ProtocolError> {
    Ok(format!("{}\n", serde_json::to_string(message)?))
}

pub fn decode_server(line: &str) -> Result<ServerMessage, ProtocolError> {
    if line.trim().is_empty() {
        return Err(ProtocolError::EmptyLine);
    }
    Ok(serde_json::from_str(line.trim_end())?)
}
