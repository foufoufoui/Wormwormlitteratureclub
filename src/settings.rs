//! Persistent player settings (text speed, volumes, language).
//! Stored as JSON in `~/.wwlc/settings.json`.

use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub text_speed: f32,   // 0..=3
    pub music_volume: f32, // 0..=1
    pub sfx_volume: f32,   // 0..=1
    pub lang: String,      // "en" | "fr"
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            text_speed: 1.0,
            music_volume: 0.8,
            sfx_volume: 0.8,
            lang: "en".to_string(),
        }
    }
}

impl GameSettings {
    pub fn char_delay_ms(&self) -> f32 {
        if self.text_speed <= 0.0 {
            0.0
        } else {
            25.0 / self.text_speed
        }
    }
}

static SETTINGS: LazyLock<Mutex<GameSettings>> =
    LazyLock::new(|| Mutex::new(load_from_disk()));

static FULLSCREEN: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

/// Toggle windowed/fullscreen; returns the new state.
pub fn toggle_fullscreen() -> bool {
    let mut g = FULLSCREEN.lock().unwrap();
    *g = !*g;
    macroquad::window::set_fullscreen(*g);
    *g
}

pub fn is_fullscreen() -> bool {
    *FULLSCREEN.lock().unwrap()
}

pub fn settings() -> GameSettings {
    SETTINGS.lock().unwrap().clone()
}

pub fn set_text_speed(v: f32) {
    let mut g = SETTINGS.lock().unwrap();
    g.text_speed = v.clamp(0.0, 3.0);
    persist(&g);
}

pub fn set_music_volume(v: f32) {
    let mut g = SETTINGS.lock().unwrap();
    g.music_volume = v.clamp(0.0, 1.0);
    persist(&g);
}

pub fn set_sfx_volume(v: f32) {
    let mut g = SETTINGS.lock().unwrap();
    g.sfx_volume = v.clamp(0.0, 1.0);
    persist(&g);
}

pub fn set_lang(l: &str) {
    let mut g = SETTINGS.lock().unwrap();
    g.lang = if l == "fr" { "fr" } else { "en" }.to_string();
    persist(&g);
}

fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wwlc")
        .join("settings.json")
}

fn load_from_disk() -> GameSettings {
    match std::fs::read_to_string(settings_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => GameSettings::default(),
    }
}

fn persist(g: &GameSettings) {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string_pretty(g) {
        let _ = std::fs::write(path, s);
    }
}
