use crate::room_manager::RoomManager;
use play_room_core::RoomId;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

pub fn schedule_round_timeout(
    manager: Arc<Mutex<RoomManager>>,
    room_id: RoomId,
    round: u32,
    deadline_ms: u64,
) {
    tokio::spawn(async move {
        let current = now_ms();
        if deadline_ms > current {
            tokio::time::sleep(Duration::from_millis(deadline_ms - current)).await;
        }
        let mut locked = manager.lock().await;
        if let Ok(messages) = locked.timeout_room(&room_id, round, now_ms()) {
            locked.flush_messages(messages);
        }
    });
}
