use play_room_core::{PlayerId, RoomId};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct RoomMemberships {
    player_rooms: BTreeMap<PlayerId, RoomId>,
}

impl RoomMemberships {
    pub fn room_for(&self, player_id: &PlayerId) -> Option<&RoomId> {
        self.player_rooms.get(player_id)
    }

    pub fn set_room(&mut self, player_id: PlayerId, room_id: RoomId) {
        self.player_rooms.insert(player_id, room_id);
    }

    pub fn remove(&mut self, player_id: &PlayerId) {
        self.player_rooms.remove(player_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_and_clears_current_room() {
        let mut memberships = RoomMemberships::default();
        let player_id = PlayerId::new("player-1");
        let room_id = RoomId::new("room-1");

        memberships.set_room(player_id.clone(), room_id.clone());
        assert_eq!(memberships.room_for(&player_id), Some(&room_id));

        memberships.remove(&player_id);
        assert_eq!(memberships.room_for(&player_id), None);
    }
}
