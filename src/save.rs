//! Save / load system. Each slot stores a full snapshot of the player state
//! (scene, name, poem tone/text, route character and flags), fixing the Java
//! version which only saved scene + name and therefore lost poem and route
//! progress on load.
//!
//! Saves live in `~/.wwlc/saves/slot_N.json`.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const NUM_SLOTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub scene: String,
    pub player_name: String,
    pub poem_tone: String,
    pub poem_text: String,
    pub route_character: Option<String>,
    pub flags: HashMap<String, String>,
    pub timestamp: String,
    /// Dialogue cursor within `scene` so a load resumes mid-scene instead of
    /// restarting it.
    #[serde(default)]
    pub cursor: usize,
    /// Background key currently displayed (if any).
    #[serde(default)]
    pub bg: Option<String>,
    /// BGM file currently playing (if any).
    #[serde(default)]
    pub music: Option<String>,
    /// Characters currently on stage.
    #[serde(default)]
    pub chars: Vec<CharSnapshot>,
    /// Current line speaker / text / typewriter position.
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub full_text: String,
    #[serde(default)]
    pub revealed: usize,
    #[serde(default)]
    pub text_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharSnapshot {
    pub name: String,
    pub expr: String,
    pub pos: String,
    pub visible: bool,
}

impl SaveData {
    pub fn is_empty(&self) -> bool {
        self.scene.is_empty()
    }
}

pub fn save(slot: usize, data: &SaveData) {
    let mut data = data.clone();
    data.timestamp = timestamp_now();
    let path = slot_path(slot);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write(path, s);
    }
}

pub fn load(slot: usize) -> SaveData {
    match std::fs::read_to_string(slot_path(slot)) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| empty()),
        Err(_) => empty(),
    }
}

pub fn delete(slot: usize) {
    let _ = std::fs::remove_file(slot_path(slot));
}

/// Load a slot and apply its full snapshot (scene, name, poem, route and
/// flags) to the global player state. Returns the data needed to resume.
pub fn load_into_state(slot: usize) -> SaveData {
    let data = load(slot);
    {
        let mut p = crate::state::player_mut();
        p.current_scene = data.scene.clone();
        p.player_name = data.player_name.clone();
        p.poem_tone = data.poem_tone.clone();
        p.poem_text = data.poem_text.clone();
        p.route_character = data.route_character.clone();
        p.flags = data.flags.clone();
    }
    data
}

pub fn load_all() -> Vec<SaveData> {
    (0..NUM_SLOTS).map(load).collect()
}

fn empty() -> SaveData {
    SaveData {
        scene: String::new(),
        player_name: "Player".to_string(),
        poem_tone: "earnest".to_string(),
        poem_text: String::new(),
        route_character: None,
        flags: HashMap::new(),
        timestamp: String::new(),
        cursor: 0,
        bg: None,
        music: None,
        chars: Vec::new(),
        speaker: None,
        full_text: String::new(),
        revealed: 0,
        text_done: true,
    }
}

fn slot_path(slot: usize) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wwlc")
        .join("saves")
        .join(format!("slot_{slot}.json"))
}

fn timestamp_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = now / 86400;
    let secs = now % 86400;
    format!("Day {}, {:02}:{:02}", days, secs / 3600, (secs % 3600) / 60)
}

/// Human readable scene titles, mirrors the Java SaveSystem list.
pub fn display_name(scene: &str) -> String {
    let names = scene_names();
    match names.get(scene) {
        Some(n) => (*n).to_string(),
        None => scene.replace('_', " "),
    }
}

fn scene_names() -> &'static HashMap<&'static str, &'static str> {
    use std::sync::LazyLock;
    static NAMES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
        let mut m = HashMap::new();
        let rows: &[(&str, &str)] = &[
            ("act1_scene1", "Act 1 - First Day"),
            ("act1_scene1b_confident", "Act 1 - First Day (confident)"),
            ("act1_scene1b_potato", "Act 1 - First Day (potato)"),
            ("act1_scene1b_stare", "Act 1 - First Day (stare)"),
            ("act1_scene2", "Act 1 - First Poem"),
            ("act1_scene3", "Act 1 - The Hymn"),
            ("act1_scene3b_hymn", "Act 1 - The Hymn (recite)"),
            ("act1_scene3b_dirtus", "Act 1 - The Hymn (Dirtus)"),
            ("act1_scene3b_mawla", "Act 1 - The Hymn (Mawla)"),
            ("act1_scene4", "Act 1 - After Club"),
            ("act1_scene5", "Act 1 - The Offering"),
            ("act1_scene5b_vermia", "Act 1 - The Offering (Vermia)"),
            ("act1_scene5b_dirtus", "Act 1 - The Offering (Dirtus)"),
            ("act1_scene5b_grubwin", "Act 1 - The Offering (Grubwin)"),
            ("act1_scene6", "Act 1 - Sunset"),
            ("act1_scene7", "Act 1 - Club Competition"),
            ("act1_scene7b_vermia", "Act 1 - Competition (Vermia)"),
            ("act1_scene7b_grubwin", "Act 1 - Competition (Grubwin)"),
            ("act1_scene7b_mawla", "Act 1 - Competition (Mawla)"),
            ("act1_scene8", "Act 1 - The Ritual"),
            ("act1_scene8b_yes", "Act 1 - The Ritual (yes)"),
            ("act1_scene8b_compromise", "Act 1 - The Ritual (compromise)"),
            ("act1_scene8b_no", "Act 1 - The Ritual (no)"),
            ("act1_scene9", "Act 1 - Worm Moon"),
            ("act2_scene1", "Act 2 - A New Day"),
            ("act2_scene1b_hymn2", "Act 2 - A New Day (hymn)"),
            ("act2_scene1b_mawla", "Act 2 - A New Day (Mawla)"),
            ("act2_scene1b_dirtus", "Act 2 - A New Day (Dirtus)"),
            ("act2_scene2", "Act 2 - The Library"),
            ("act2_scene3", "Act 2 - Rooftop"),
            ("act2_scene4", "Act 2 - The Invitation"),
            ("act2_scene4b_yes", "Act 2 - The Invitation (yes)"),
            ("act2_scene4b_questions", "Act 2 - The Invitation (questions)"),
            ("act2_scene4b_hesitate", "Act 2 - The Invitation (hesitate)"),
            ("act2_scene5", "Act 2 - Second Poem"),
            ("act2_scene6", "Act 2 - After the Poem"),
            ("act2_scene7", "Act 2 - Revelations"),
            ("act2_scene8", "Act 2 - Underground"),
            ("act3_scene1", "Act 3 - The Change"),
            ("act3_scene2", "Act 3 - Unraveling"),
            ("act3_scene3", "Act 3 - Breaking Point"),
            ("act3_scene4", "Act 3 - The Truth"),
            ("act3_scene5", "Act 3 - End of Days"),
            ("act4_scene1", "Act 4 - The Presentation"),
            ("act4_scene2", "Act 4 - Final Words"),
            ("act4_ending_true", "Act 4 - True Ending"),
            ("act4_ending_potato", "Act 4 - Potato Ending"),
        ];
        for (k, v) in rows {
            m.insert(*k, *v);
        }
        m
    });
    &NAMES
}
