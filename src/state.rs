//! Runtime player state shared across screens: name, scene, poem data,
//! route character and story flags. The Java version declared these fields
//! but never wired them together; here they are fully functional and are
//! part of every save file.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub player_name: String,
    pub current_scene: String,
    pub poem_tone: String, // "earnest" | "sardonic" | "tender"
    pub poem_text: String,
    pub route_character: Option<String>, // "vermia" | "grubwin" | "mawla"
    pub flags: HashMap<String, String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            player_name: "Player".to_string(),
            current_scene: "act1_scene1".to_string(),
            poem_tone: "earnest".to_string(),
            poem_text: String::new(),
            route_character: None,
            flags: HashMap::new(),
        }
    }
}

impl PlayerState {
    pub fn set_name(&mut self, name: &str) {
        let trimmed = name.trim();
        self.player_name = if trimmed.is_empty() {
            "Player".to_string()
        } else {
            trimmed.to_string()
        };
    }

    pub fn set_flag(&mut self, flag: &str, value: &str) {
        self.flags.insert(flag.to_string(), value.to_string());
    }

    pub fn flag(&self, flag: &str) -> Option<&String> {
        self.flags.get(flag)
    }
}

static PLAYER: LazyLock<Mutex<PlayerState>> = LazyLock::new(|| Mutex::new(PlayerState::default()));

pub fn player() -> PlayerState {
    PLAYER.lock().unwrap().clone()
}

pub fn player_mut() -> std::sync::MutexGuard<'static, PlayerState> {
    PLAYER.lock().unwrap()
}

pub fn reset() {
    *PLAYER.lock().unwrap() = PlayerState::default();
}
