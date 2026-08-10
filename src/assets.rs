//! Asset loading: fonts, background textures, animated GIF menu background,
//! character sprites and audio.
//!
//! macroquad's stable audio stack cannot decode MP3, so every `.mp3` in
//! `assets/audio/` is decoded with a pure-Rust MP3 decoder and re-encoded as
//! an in-memory PCM WAV before being handed to macroquad. This fixes the Java
//! game's missing `text_blip` sound by synthesising one, and degrades
//! gracefully (with fallbacks) for the missing `clubroom_ritual` track.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::{LazyLock, Mutex};

use macroquad::audio::{self, Sound};
use macroquad::prelude::*;
use macroquad::text::get_default_font;

static ASSETS: LazyLock<Mutex<Option<Arc<Assets>>>> = LazyLock::new(|| Mutex::new(None));

/// Install the fully-loaded assets (textures, fonts) for the rest of the app.
pub fn init(assets: Assets) {
    *ASSETS.lock().unwrap() = Some(Arc::new(assets));
}

/// Return the global assets. The `Arc` is cheap to clone and lets us drop the
/// lock immediately, so nested `assets()` calls (e.g. a screen helper re-fetch
/// while its parent holds a reference) can never deadlock.
pub fn assets() -> Arc<Assets> {
    ASSETS
        .lock()
        .unwrap()
        .clone()
        .expect("assets not initialised")
}

pub struct Fonts {
    pub regular: Font,
    pub bold: Font,
    pub italic: Font,
}

pub struct MenuFrame {
    pub img: Image,
    pub delay_ms: u32,
}

pub struct Assets {
    pub fonts: Fonts,
    pub bg: HashMap<String, Texture2D>,
    pub chars: HashMap<String, Texture2D>,
    /// Opaque (non-transparent) bounding box of each character sprite, in
    /// source-pixels. The PNGs are wide canvases with transparent margins, so
    /// this lets the game draw just the character at its native aspect ratio
    /// instead of squishing the whole canvas.
    pub chars_bbox: HashMap<String, Rect>,
    pub sounds: HashMap<String, Arc<Sound>>,
    pub menu_frames: Vec<MenuFrame>,
    pub confirm_bg: Option<Texture2D>,
}

impl Assets {
    pub async fn load() -> Assets {
        // Yield between phases so the window stays responsive (and a splash
        // screen is visible) while assets are decoded. Without this the
        // synchronous GIF/MP3 decoding blocks the event loop before the first
        // frame is ever presented, freezing a blank window.
        let fonts = load_fonts().await;
        next_frame().await;

        let mut bg = HashMap::new();
        for name in ALL_BACKGROUNDS {
            if let Some(tex) = load_texture_asset(&format!("backgrounds/{name}.png")) {
                tex.set_filter(FilterMode::Linear);
                bg.insert(name.to_string(), tex);
            }
        }
        next_frame().await;

        let mut chars = HashMap::new();
        let mut chars_bbox = HashMap::new();
        for root in character_roots() {
            if let Ok(entries) = std::fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if !entry.path().is_dir() {
                        continue;
                    }
                    if let Ok(files) = std::fs::read_dir(entry.path()) {
                        for f in files.flatten() {
                            let fname = f.file_name().to_string_lossy().to_lowercase();
                            if !fname.ends_with(".png") {
                                continue;
                            }
                            let expr = fname.trim_end_matches(".png").to_string();
                            let rel = format!("characters/{name}/{expr}.png");
                            if let Some((tex, bbox)) = load_char_texture(&rel) {
                                tex.set_filter(FilterMode::Linear);
                                chars.insert(format!("{name}/{expr}"), tex);
                                chars_bbox.insert(format!("{name}/{expr}"), bbox);
                            }
                        }
                    }
                }
            }
        }
        next_frame().await;

        let confirm_bg = load_texture_asset("backgrounds/confirm_bg.png");
        let menu_frames = load_menu_frames().await;
        next_frame().await;

        let mut sounds = HashMap::new();
        for name in ALL_AUDIO {
            if let Some(sound) = load_audio(name).await {
                sounds.insert(name.to_string(), Arc::new(sound));
            }
        }
        if let Some(blip) = synthesize_blip().await {
            sounds.insert("text_blip".to_string(), Arc::new(blip));
        }

        Assets {
            fonts,
            bg,
            chars,
            chars_bbox,
            sounds,
            menu_frames,
            confirm_bg,
        }
    }

    /// Texture + opaque source rect for a character, falling back to its
    /// `normal` expression when the requested one has no art yet.
    pub fn char_frame(&self, name: &str, expr: &str) -> Option<(Texture2D, Rect)> {
        let key = format!("{}/{}", name.to_lowercase(), expr.to_lowercase());
        if let Some(t) = self.chars.get(&key).cloned() {
            let bbox = self.bbox(&key, &t);
            return Some((t, bbox));
        }
        let key = format!("{}/normal", name.to_lowercase());
        let t = self.chars.get(&key).cloned()?;
        let bbox = self.bbox(&key, &t);
        Some((t, bbox))
    }

    fn bbox(&self, key: &str, tex: &Texture2D) -> Rect {
        self.chars_bbox.get(key).copied().unwrap_or_else(|| {
            Rect::new(0.0, 0.0, tex.width(), tex.height())
        })
    }
}

const ALL_BACKGROUNDS: &[&str] = &[
    "classroom",
    "clubroom_day",
    "clubroom_night",
    "clubroom_ritual",
    "clubroom_sunset",
    "hallway",
    "park_day",
    "park_scatha",
    "park_sunset",
    "rooftop",
    "underground",
];

const ALL_AUDIO: &[&str] = &[
    "clubroom_hymn",
    "clubroom_normal",
    "clubroom_normal2",
    "ending_credits",
    "menu_select",
    "poem_writing",
    "scatha_bless",
    "scatha_theme",
    "tension",
];

// ── Fonts ─────────────────────────────────────────────────────────────────

/// Load a texture from `assets/<rel>`, resolving the path ourselves.
/// macroquad's `load_texture` only resolves relative to the process working
/// directory, which silently fails when the binary is launched from anywhere
/// else (our `read_asset` additionally checks `CARGO_MANIFEST_DIR`).
fn load_texture_asset(rel: &str) -> Option<Texture2D> {
    let bytes = read_asset(rel)?;
    let rgba = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = (rgba.width() as u16, rgba.height() as u16);
    let img = Image {
        bytes: rgba.into_raw(),
        width: w,
        height: h,
    };
    Some(Texture2D::from_image(&img))
}

/// Load a character sprite plus the bounding box of its visible pixels.
fn load_char_texture(rel: &str) -> Option<(Texture2D, Rect)> {
    let bytes = read_asset(rel)?;
    let rgba = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = (rgba.width() as u16, rgba.height() as u16);
    let img = Image {
        bytes: rgba.into_raw(),
        width: w,
        height: h,
    };
    let bbox = opaque_bbox(&img);
    Some((Texture2D::from_image(&img), bbox))
}

/// Bounding box of the pixels with meaningful alpha in a sprite image.
fn opaque_bbox(img: &Image) -> Rect {
    let w = img.width as usize;
    let h = img.height as usize;
    let bytes = &img.bytes;
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for y in 0..h {
        for x in 0..w {
            if bytes[(y * w + x) * 4 + 3] > 8 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if max_x < min_x {
        return Rect::new(0.0, 0.0, img.width as f32, img.height as f32);
    }
    Rect::new(
        min_x as f32,
        min_y as f32,
        (max_x - min_x + 1) as f32,
        (max_y - min_y + 1) as f32,
    )
}

/// Candidate `assets/characters` directories (cwd, CARGO_MANIFEST_DIR, then
/// next to the executable for packaged installs).
fn character_roots() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("assets").join("characters"));
    }
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        out.push(std::path::PathBuf::from(dir).join("assets").join("characters"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("assets").join("characters"));
            out.push(dir.join("..").join("Resources").join("assets").join("characters"));
        }
    }
    out
}

async fn load_fonts() -> Fonts {
    let (reg_path, bold_path, ital_path) = font_candidates();
    let reg = load_font(&reg_path).await.unwrap_or_else(get_default_font);
    let bold = match &bold_path {
        Some(p) => load_font(p).await.unwrap_or_else(|| reg.clone()),
        None => reg.clone(),
    };
    let ital = match &ital_path {
        Some(p) => load_font(p).await.unwrap_or_else(|| reg.clone()),
        None => reg.clone(),
    };
    Fonts {
        regular: reg,
        bold,
        italic: ital,
    }
}

async fn load_font(path: &str) -> Option<Font> {
    match std::fs::read(path) {
        Ok(bytes) => load_ttf_font_from_bytes(&bytes).ok(),
        Err(_) => None,
    }
}

fn font_candidates() -> (String, Option<String>, Option<String>) {
    const CANDIDATES: &[(&str, Option<&str>, Option<&str>)] = &[
        // macOS
        ("/System/Library/Fonts/Supplemental/Georgia.ttf",
         Some("/System/Library/Fonts/Supplemental/Georgia Bold.ttf"),
         Some("/System/Library/Fonts/Supplemental/Georgia Italic.ttf")),
        ("/Library/Fonts/Georgia.ttf",
         Some("/Library/Fonts/Georgia Bold.ttf"),
         Some("/Library/Fonts/Georgia Italic.ttf")),
        // Windows
        ("C:/Windows/Fonts/georgia.ttf",
         Some("C:/Windows/Fonts/georgiab.ttf"),
         Some("C:/Windows/Fonts/georgiai.ttf")),
        // Linux
        ("/usr/share/fonts/truetype/msttcorefonts/Georgia.ttf", None, None),
        ("/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf", None, None),
    ];
    for (reg, bold, ital) in CANDIDATES {
        if std::path::Path::new(reg).exists() {
            return (reg.to_string(), bold.map(|b| b.to_string()), ital.map(|i| i.to_string()));
        }
    }
    (String::new(), None, None)
}

// ── Audio ─────────────────────────────────────────────────────────────────

async fn load_audio(name: &str) -> Option<Sound> {
    for ext in ["mp3", "ogg", "wav"] {
        let path = format!("audio/{name}.{ext}");
        if let Some(bytes) = read_asset(&path) {
            let data = if ext == "mp3" {
                mp3_to_wav(&bytes)?
            } else {
                bytes
            };
            if let Ok(sound) = audio::load_sound_from_bytes(&data).await {
                return Some(sound);
            }
        }
    }
    crate::log::warn(&format!("[audio] missing: {name}"));
    None
}

/// Decode an MP3 file into a 16-bit PCM WAV byte buffer.
fn mp3_to_wav(mp3: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = minimp3::Decoder::new(Cursor::new(mp3));
    let mut samples: Vec<i16> = Vec::new();
    let mut channels = 2;
    let mut sample_rate = 44100u32;
    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                channels = frame.channels;
                sample_rate = frame.sample_rate as u32;
                samples.extend_from_slice(&frame.data);
            }
            Err(minimp3::Error::Eof) => break,
            Err(_) => return None,
        }
    }
    if samples.is_empty() {
        return None;
    }
    Some(wav_pcm16(&samples, channels, sample_rate))
}

/// Synthesise the missing `text_blip` typing sound (short soft click).
async fn synthesize_blip() -> Option<Sound> {
    const RATE: u32 = 44100;
    let dur = 0.035f32;
    let n = (RATE as f32 * dur) as usize;
    let mut samples = Vec::with_capacity(n * 2);
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let env = (-t * 140.0).exp();
        let v = (t * std::f32::consts::TAU * 1180.0).sin() * env * 0.28;
        let s = (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        samples.push(s);
        samples.push(s);
    }
    let wav = wav_pcm16(&samples, 2, RATE);
    audio::load_sound_from_bytes(&wav).await.ok()
}

fn wav_pcm16(samples: &[i16], channels: usize, sample_rate: u32) -> Vec<u8> {
    let bits = 16u16;
    let block_align = (channels as u16) * (bits / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&(channels as u16).to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }
    wav
}

// ── Animated GIF menu background ──────────────────────────────────────────

/// Decode the animated menu background into CPU-side frames at native
/// resolution. Frames are streamed one at a time (keeps peak memory down) and
/// uploaded to the GPU lazily by the menu, keeping both VRAM and load time low.
async fn load_menu_frames() -> Vec<MenuFrame> {
    let Some(bytes) = read_asset("backgrounds/menu_bg.gif") else {
        return Vec::new();
    };
    use image::AnimationDecoder;
    let decoder = match image::codecs::gif::GifDecoder::new(Cursor::new(&bytes)) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for frame in decoder.into_frames() {
        let Ok(frame) = frame else { break };
        let buf = frame.buffer();
        let (delay_ms, _) = frame.delay().numer_denom_ms();
        let img = Image {
            bytes: buf.as_raw().to_vec(),
            width: buf.width() as u16,
            height: buf.height() as u16,
        };
        out.push(MenuFrame { img, delay_ms });
        if out.len() % 16 == 0 {
            next_frame().await;
        }
    }
    out
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Read a file relative to the `assets` folder, trying cwd then CARGO_MANIFEST_DIR.
pub fn read_asset(rel: &str) -> Option<Vec<u8>> {
    for path in crate::dialogue::asset_candidates(rel) {
        if let Ok(bytes) = std::fs::read(&path) {
            return Some(bytes);
        }
    }
    None
}
