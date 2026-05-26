use play_room_core::{PlayerId, SessionToken};

#[derive(Clone, Debug)]
pub struct ScriptedClient {
    pub name: String,
    pub player_id: Option<PlayerId>,
    pub reconnect_token: Option<SessionToken>,
}

impl ScriptedClient {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            player_id: None,
            reconnect_token: None,
        }
    }
}
