use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const SLOW_DRAFT_IO_THRESHOLD: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Draft {
    #[serde(default)]
    pub text: String,
}

pub fn load() -> Draft {
    let start = Instant::now();
    match confy::load("grammy", "draft") {
        Ok(draft) => {
            let elapsed = start.elapsed();
            if elapsed >= SLOW_DRAFT_IO_THRESHOLD {
                eprintln!("[DEBUG] Draft load completed in {:?}", elapsed);
            }
            draft
        }
        Err(err) => {
            eprintln!(
                "[DEBUG] Failed to load draft after {:?}: {}",
                start.elapsed(),
                err
            );
            Draft::default()
        }
    }
}

pub fn save_text(text: String) {
    let draft = Draft { text };
    let start = Instant::now();
    match confy::store("grammy", "draft", draft) {
        Ok(()) => {
            let elapsed = start.elapsed();
            if elapsed >= SLOW_DRAFT_IO_THRESHOLD {
                eprintln!("[DEBUG] Draft save completed in {:?}", elapsed);
            }
        }
        Err(err) => {
            eprintln!(
                "[DEBUG] Failed to save draft after {:?}: {}",
                start.elapsed(),
                err
            );
        }
    }
}
