use crate::fanout::OutboundMessages;
use crate::room_manager::{RoomManager, SeatExpiry, SpectatorExpiry};
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
        let messages = {
            let mut locked = manager.lock().await;
            locked.timeout_room(&room_id, round, now_ms()).ok()
        };
        if let Some(messages) = messages {
            flush_and_schedule(manager, messages).await;
        }
    });
}

pub async fn flush_and_schedule(manager: Arc<Mutex<RoomManager>>, messages: OutboundMessages) {
    let mut pending = messages;
    let mut seat_expiries = Vec::new();
    let mut spectator_expiries = Vec::new();

    while !pending.is_empty() {
        let outcomes = {
            let mut locked = manager.lock().await;
            locked.flush_messages(pending, now_ms())
        };
        pending = Vec::new();

        for outcome in outcomes {
            pending.extend(outcome.messages);
            if let Some(expiry) = outcome.seat_expiry {
                seat_expiries.push(expiry);
            }
            if let Some(expiry) = outcome.spectator_expiry {
                spectator_expiries.push(expiry);
            }
        }
    }

    for expiry in seat_expiries {
        schedule_seat_expiry(manager.clone(), expiry);
    }
    for expiry in spectator_expiries {
        schedule_spectator_expiry(manager.clone(), expiry);
    }
}

pub fn schedule_seat_expiry(manager: Arc<Mutex<RoomManager>>, expiry: SeatExpiry) {
    tokio::spawn(async move {
        let current = now_ms();
        if expiry.expires_at_ms > current {
            tokio::time::sleep(Duration::from_millis(expiry.expires_at_ms - current)).await;
        }
        let spectator_expiry = {
            let mut locked = manager.lock().await;
            match locked.expire_participant_seat(&expiry) {
                Ok(outcome) => {
                    let messages = outcome.messages;
                    drop(locked);
                    flush_and_schedule(manager.clone(), messages).await;
                    outcome.spectator_expiry
                }
                Err(_) => None,
            }
        };
        if let Some(expiry) = spectator_expiry {
            schedule_spectator_expiry(manager.clone(), expiry);
        }
    });
}

pub fn schedule_spectator_expiry(manager: Arc<Mutex<RoomManager>>, expiry: SpectatorExpiry) {
    tokio::spawn(async move {
        let current = now_ms();
        if expiry.expires_at_ms > current {
            tokio::time::sleep(Duration::from_millis(expiry.expires_at_ms - current)).await;
        }
        let messages = {
            let mut locked = manager.lock().await;
            locked.expire_spectator(&expiry).ok()
        };
        if let Some(messages) = messages {
            flush_and_schedule(manager, messages).await;
        }
    });
}
