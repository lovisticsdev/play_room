use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Deadline {
    pub expires_at_ms: u64,
}

impl Deadline {
    pub fn from_now(now_ms: u64, duration_ms: u64) -> Self {
        Self {
            expires_at_ms: now_ms.saturating_add(duration_ms),
        }
    }

    pub fn is_expired(self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }
}
