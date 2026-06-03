use play_room_core::{GameRoom, RoomId, RoomSummary};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoomLookupError {
    NotFound(String),
    Ambiguous(String),
}

#[derive(Default)]
pub struct RoomRegistry {
    rooms: BTreeMap<RoomId, GameRoom>,
}

impl RoomRegistry {
    pub fn get(&self, room_id: &RoomId) -> Option<&GameRoom> {
        self.rooms.get(room_id)
    }

    pub fn get_mut(&mut self, room_id: &RoomId) -> Option<&mut GameRoom> {
        self.rooms.get_mut(room_id)
    }

    pub fn insert(&mut self, room: GameRoom) -> Option<GameRoom> {
        self.rooms.insert(room.id().clone(), room)
    }

    pub fn remove(&mut self, room_id: &RoomId) -> Option<GameRoom> {
        self.rooms.remove(room_id)
    }

    pub fn summaries(&self) -> Vec<RoomSummary> {
        self.rooms.values().map(GameRoom::summary).collect()
    }

    pub fn resolve_room_id(&self, room_id_or_name: &RoomId) -> Result<RoomId, RoomLookupError> {
        if self.rooms.contains_key(room_id_or_name) {
            return Ok(room_id_or_name.clone());
        }

        let requested = room_id_or_name.as_str().trim();
        let matches: Vec<RoomId> = self
            .rooms
            .iter()
            .filter(|(_, room)| room.name().trim().eq_ignore_ascii_case(requested))
            .map(|(room_id, _)| room_id.clone())
            .collect();

        match matches.as_slice() {
            [room_id] => Ok(room_id.clone()),
            [] => Err(RoomLookupError::NotFound(requested.to_owned())),
            _ => Err(RoomLookupError::Ambiguous(requested.to_owned())),
        }
    }

    pub fn room_name_exists(&self, name: &str) -> bool {
        self.rooms
            .values()
            .any(|room| room.name().trim().eq_ignore_ascii_case(name.trim()))
    }

    pub fn suggest_room_names(&self, desired: &str, owner_name: Option<&str>) -> Vec<String> {
        let base = slugify(desired);
        let mut suggestions = Vec::new();
        self.push_available_room_name(&mut suggestions, format!("{base}-2"));

        if let Some(owner_slug) = owner_name
            .map(slugify)
            .filter(|slug| !slug.is_empty() && slug != &base)
        {
            self.push_available_room_name(&mut suggestions, format!("{base}-{owner_slug}"));
        }

        let mut suffix = 3;
        while suggestions.len() < 3 {
            self.push_available_room_name(&mut suggestions, format!("{base}-{suffix}"));
            suffix += 1;
        }
        suggestions
    }

    pub fn should_remove_room(&self, room_id: &RoomId) -> bool {
        self.get(room_id)
            .map(|room| room.player_ids().is_empty())
            .unwrap_or(false)
    }

    fn push_available_room_name(&self, names: &mut Vec<String>, candidate: String) {
        if !names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&candidate))
            && !self.room_name_exists(&candidate)
        {
            names.push(candidate);
        }
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "room".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use play_room_core::{GameRules, Player, PlayerId, RoomCommand};

    fn room(id: &str, name: &str) -> GameRoom {
        GameRoom::new(
            RoomId::new(id),
            name.to_owned(),
            GameRules::default(),
            Player::participant(PlayerId::new(format!("host-{id}")), "host"),
        )
        .unwrap()
    }

    #[test]
    fn stores_and_lists_room_summaries() {
        let mut registry = RoomRegistry::default();
        registry.insert(room("room-1", "TestRoom"));

        let summaries = registry.summaries();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, RoomId::new("room-1"));
        assert_eq!(summaries[0].name, "TestRoom");
    }

    #[test]
    fn resolves_room_by_id_or_case_insensitive_name() {
        let mut registry = RoomRegistry::default();
        registry.insert(room("room-1", "TestRoom"));

        assert_eq!(
            registry.resolve_room_id(&RoomId::new("room-1")),
            Ok(RoomId::new("room-1"))
        );
        assert_eq!(
            registry.resolve_room_id(&RoomId::new("testroom")),
            Ok(RoomId::new("room-1"))
        );
    }

    #[test]
    fn suggests_available_room_names() {
        let mut registry = RoomRegistry::default();
        registry.insert(room("room-1", "test-room-2"));

        assert_eq!(
            registry.suggest_room_names("Test Room", Some("Alice Smith")),
            vec!["test-room-alice-smith", "test-room-3", "test-room-4"]
        );
    }

    #[test]
    fn detects_empty_rooms_for_cleanup() {
        let mut registry = RoomRegistry::default();
        let room_id = RoomId::new("room-1");
        registry.insert(room("room-1", "TestRoom"));

        assert!(!registry.should_remove_room(&room_id));

        registry
            .get_mut(&room_id)
            .unwrap()
            .apply(RoomCommand::Leave {
                player_id: PlayerId::new("host-room-1"),
            })
            .unwrap();

        assert!(registry.should_remove_room(&room_id));
        assert!(registry.remove(&room_id).is_some());
        assert!(registry.get(&room_id).is_none());
    }
}
