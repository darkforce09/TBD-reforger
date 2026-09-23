use super::*;
const TOAST_SECS: u64 = 6;

// ---- toasts ----

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub until: Instant,
}

impl Toast {
    pub fn new(text: String, error: bool) -> Self {
        Self {
            text,
            error,
            until: Instant::now() + std::time::Duration::from_secs(TOAST_SECS),
        }
    }
}
