//! BGM / SFX playback on top of the preloaded sound assets.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use macroquad::audio::{self, PlaySoundParams, Sound};

use crate::settings;

/// Fallback map for tracks referenced by the story but not shipped.
/// `clubroom_ritual` is referenced by Act 4 but missing from `assets/audio/`;
/// we substitute the thematically closest ambient track so the scene still
/// has music instead of silence.
const FALLBACKS: &[(&str, &str)] = &[("clubroom_ritual", "clubroom_hymn")];

struct AudioState {
    sounds: HashMap<String, Arc<Sound>>,
    current_bgm: Option<String>,
    last_sfx: HashMap<String, f64>,
}

static AUDIO: LazyLock<Mutex<AudioState>> = LazyLock::new(|| {
    Mutex::new(AudioState {
        sounds: HashMap::new(),
        current_bgm: None,
        last_sfx: HashMap::new(),
    })
});

/// Initialise the audio layer with the fully-loaded sounds.
pub fn init(sounds: HashMap<String, Arc<Sound>>) {
    let mut a = AUDIO.lock().unwrap();
    a.sounds = sounds;
    a.current_bgm = None;
    a.last_sfx.clear();
}

fn resolve<'a>(name: &'a str) -> &'a str {
    for (from, to) in FALLBACKS {
        if *from == name {
            return to;
        }
    }
    name
}

pub fn play_bgm(name: &str) {
    // Randomly pick clubroom_normal2 for variety, like the original.
    let name = if name == "clubroom_normal" && macroquad::rand::gen_range(0u32, 100) > 50 {
        "clubroom_normal2"
    } else {
        name
    };
    let mut a = AUDIO.lock().unwrap();
    if a.current_bgm.as_deref() == Some(name) {
        return;
    }
    if let Some(old) = a.current_bgm.take() {
        if let Some(sound) = a.sounds.get(&old) {
            audio::stop_sound(sound);
        }
    }
    let Some(sound) = a.sounds.get(resolve(name)) else {
        crate::log::warn(&format!("[audio] BGM not found: {name}"));
        return;
    };
    let vol = settings::settings().music_volume;
    audio::play_sound(
        sound,
        PlaySoundParams {
            looped: true,
            volume: vol,
        },
    );
    a.current_bgm = Some(name.to_string());
}

pub fn stop_bgm() {
    let mut a = AUDIO.lock().unwrap();
    if let Some(old) = a.current_bgm.take() {
        if let Some(sound) = a.sounds.get(&old) {
            audio::stop_sound(sound);
        }
    }
}

/// The name of the track currently playing, if any.
pub fn current_bgm() -> Option<String> {
    AUDIO.lock().unwrap().current_bgm.clone()
}

pub fn set_bgm_volume(v: f32) {
    let a = AUDIO.lock().unwrap();
    if let Some(name) = &a.current_bgm {
        if let Some(sound) = a.sounds.get(name) {
            audio::set_sound_volume(sound, v);
        }
    }
}

/// Rate-limit repeat SFX (the typing blip) to avoid a wall of overlapping
/// sounds when revealing text very fast.
pub fn play_sfx(name: &str) {
    let now = macroquad::time::get_time();
    let sound = {
        let mut a = AUDIO.lock().unwrap();
        let last = a.last_sfx.entry(name.to_string()).or_insert(0.0);
        if name == "text_blip" && now - *last < 0.018 {
            return;
        }
        *last = now;
        a.sounds.get(resolve(name)).cloned()
    };
    let Some(sound) = sound else {
        return;
    };
    let vol = settings::settings().sfx_volume;
    audio::play_sound(
        &sound,
        PlaySoundParams {
            looped: false,
            volume: vol,
        },
    );
}
