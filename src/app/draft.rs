use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Draft {
    #[serde(default)]
    pub text: String,
}

pub fn load() -> Draft {
    confy::load("grammy", "draft").unwrap_or_default()
}

pub fn save_text(text: String) {
    let draft = Draft { text };
    let _ = confy::store("grammy", "draft", draft);
}
