use play_room_core::{
    GameRoom, Move, Player, PlayerId, PlayerRole, RoomCommand, RoomId, RoomPhase,
};
use play_room_testkit::{Scenario, ScenarioStep};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const SCENARIO_DIR: &str = "examples/scripted_clients";

#[test]
fn scripted_client_examples_are_executable() {
    let mut paths = fs::read_dir(SCENARIO_DIR)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();

    assert!(!paths.is_empty(), "expected scripted scenario fixtures");

    for path in paths {
        let scenario = load_scenario(&path);
        let mut world = ScenarioWorld::default();
        world.run(&scenario).unwrap_or_else(|err| {
            panic!("{} failed: {err}", path.display());
        });
    }
}

fn load_scenario(path: &Path) -> Scenario {
    let text = fs::read_to_string(path).unwrap();
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("{} is invalid: {err}", path.display()))
}

#[derive(Clone, Debug)]
struct ScenarioClient {
    player_id: PlayerId,
    current_room: Option<RoomId>,
    connected: bool,
}

#[derive(Default)]
struct ScenarioWorld {
    clients: BTreeMap<String, ScenarioClient>,
    rooms: BTreeMap<RoomId, GameRoom>,
    now_ms: u64,
    next_room: usize,
}

impl ScenarioWorld {
    fn run(&mut self, scenario: &Scenario) -> Result<(), String> {
        for step in &scenario.steps {
            self.apply(step)?;
        }
        Ok(())
    }

    fn apply(&mut self, step: &ScenarioStep) -> Result<(), String> {
        match step {
            ScenarioStep::Connect { client } => self.connect(client),
            ScenarioStep::CreateRoom { client, name } => self.create_room(client, name),
            ScenarioStep::JoinRoom { client, room_id } => {
                self.join_room(client, room_id, PlayerRole::Participant)
            }
            ScenarioStep::Ready { client } => self.ready(client),
            ScenarioStep::Move { client, mv } => self.submit_move(client, *mv),
            ScenarioStep::Spectate { client, room_id } => {
                if let Some(room_id) = room_id {
                    self.join_room(client, room_id, PlayerRole::Spectator)
                } else {
                    self.set_spectator(client)
                }
            }
            ScenarioStep::WaitMs { ms } => self.wait(*ms),
            ScenarioStep::Disconnect { client } => self.disconnect(client),
            ScenarioStep::Reconnect { client } => self.reconnect(client),
        }
    }

    fn connect(&mut self, client: &str) -> Result<(), String> {
        if self.clients.contains_key(client) {
            return Err(format!("client already connected: {client}"));
        }
        self.clients.insert(
            client.to_owned(),
            ScenarioClient {
                player_id: player_id(client),
                current_room: None,
                connected: true,
            },
        );
        Ok(())
    }

    fn create_room(&mut self, client: &str, name: &str) -> Result<(), String> {
        let player_id = self.client(client)?.player_id.clone();
        self.leave_current_room(client)?;

        self.next_room += 1;
        let room_id = RoomId::new(format!("room-{}", self.next_room));
        let host = Player::participant(player_id, client);
        let room = GameRoom::new(room_id.clone(), name, Default::default(), host)
            .map_err(|err| err.to_string())?;
        self.rooms.insert(room_id.clone(), room);
        self.client_mut(client)?.current_room = Some(room_id);
        Ok(())
    }

    fn join_room(
        &mut self,
        client: &str,
        room_ref: &RoomId,
        role: PlayerRole,
    ) -> Result<(), String> {
        let room_id = self.resolve_room(room_ref)?;
        let player_id = self.client(client)?.player_id.clone();
        let player = match role {
            PlayerRole::Participant => Player::participant(player_id, client),
            PlayerRole::Spectator => Player::spectator(player_id, client),
        };

        self.leave_current_room(client)?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| format!("room not found: {room_id}"))?;
        room.apply(RoomCommand::Join { player })
            .map_err(|err| err.to_string())?;
        self.client_mut(client)?.current_room = Some(room_id);
        Ok(())
    }

    fn ready(&mut self, client: &str) -> Result<(), String> {
        let (room_id, player_id) = self.room_and_player(client)?;
        self.rooms
            .get_mut(&room_id)
            .unwrap()
            .apply(RoomCommand::SetReady {
                player_id,
                ready: true,
                now_ms: self.now_ms,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    fn submit_move(&mut self, client: &str, mv: Move) -> Result<(), String> {
        let (room_id, player_id) = self.room_and_player(client)?;
        self.rooms
            .get_mut(&room_id)
            .unwrap()
            .apply(RoomCommand::SubmitMove {
                player_id,
                mv,
                now_ms: self.now_ms,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    fn set_spectator(&mut self, client: &str) -> Result<(), String> {
        let (room_id, player_id) = self.room_and_player(client)?;
        self.rooms
            .get_mut(&room_id)
            .unwrap()
            .apply(RoomCommand::SetSpectator {
                player_id,
                spectator: true,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    fn wait(&mut self, ms: u64) -> Result<(), String> {
        self.now_ms = self.now_ms.saturating_add(ms);
        let due = self
            .rooms
            .iter()
            .filter_map(|(room_id, room)| match room.phase() {
                RoomPhase::InRound { round, deadline_ms } if *deadline_ms <= self.now_ms => {
                    Some((room_id.clone(), *round))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        for (room_id, round) in due {
            self.rooms
                .get_mut(&room_id)
                .unwrap()
                .apply(RoomCommand::TimeoutRound {
                    round,
                    now_ms: self.now_ms,
                })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn disconnect(&mut self, client: &str) -> Result<(), String> {
        let (room_id, player_id) = self.room_and_player(client)?;
        self.rooms
            .get_mut(&room_id)
            .unwrap()
            .apply(RoomCommand::Disconnect { player_id })
            .map_err(|err| err.to_string())?;
        self.client_mut(client)?.connected = false;
        Ok(())
    }

    fn reconnect(&mut self, client: &str) -> Result<(), String> {
        let (room_id, player_id) = self.room_and_player(client)?;
        self.rooms
            .get_mut(&room_id)
            .unwrap()
            .apply(RoomCommand::Reconnect { player_id })
            .map_err(|err| err.to_string())?;
        self.client_mut(client)?.connected = true;
        Ok(())
    }

    fn leave_current_room(&mut self, client: &str) -> Result<(), String> {
        let Some(room_id) = self.client(client)?.current_room.clone() else {
            return Ok(());
        };
        let player_id = self.client(client)?.player_id.clone();
        if let Some(room) = self.rooms.get_mut(&room_id) {
            room.apply(RoomCommand::Leave { player_id })
                .map_err(|err| err.to_string())?;
        }
        self.client_mut(client)?.current_room = None;
        Ok(())
    }

    fn resolve_room(&self, room_ref: &RoomId) -> Result<RoomId, String> {
        if self.rooms.contains_key(room_ref) {
            return Ok(room_ref.clone());
        }

        let matches = self
            .rooms
            .iter()
            .filter(|(_, room)| room.name() == room_ref.as_str())
            .map(|(room_id, _)| room_id.clone())
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [room_id] => Ok(room_id.clone()),
            [] => Err(format!("room not found: {room_ref}")),
            _ => Err(format!("multiple rooms named {room_ref}")),
        }
    }

    fn room_and_player(&self, client: &str) -> Result<(RoomId, PlayerId), String> {
        let client_state = self.client(client)?;
        let room_id = client_state
            .current_room
            .clone()
            .ok_or_else(|| format!("client is not in a room: {client}"))?;
        Ok((room_id, client_state.player_id.clone()))
    }

    fn client(&self, client: &str) -> Result<&ScenarioClient, String> {
        self.clients
            .get(client)
            .ok_or_else(|| format!("unknown client: {client}"))
    }

    fn client_mut(&mut self, client: &str) -> Result<&mut ScenarioClient, String> {
        self.clients
            .get_mut(client)
            .ok_or_else(|| format!("unknown client: {client}"))
    }
}

fn player_id(client: &str) -> PlayerId {
    PlayerId::new(format!("player-{client}"))
}
