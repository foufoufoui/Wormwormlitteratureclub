//! The main menu and the name-entry screen.

use macroquad::prelude::*;

use crate::assets::{self, Fonts};
use crate::event::ScreenEvent;
use crate::overlay::{self, SaveMode, SettingsResult};
use crate::ui::{self, ButtonStyle, W, H};
use crate::{audio, lang, save, state};

pub struct Menu {
    pub overlay: Option<MenuOverlay>,
    pub confirm_overwrite: Option<usize>,
    elapsed: f32,
    frame_idx: usize,
    bg_tex: Option<Texture2D>,
    bg_tex_idx: usize,
}

pub enum MenuOverlay {
    Load,
    Settings,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            overlay: None,
            confirm_overwrite: None,
            elapsed: 0.0,
            frame_idx: 0,
            bg_tex: None,
            bg_tex_idx: usize::MAX,
        }
    }

    pub fn frame(&mut self) -> ScreenEvent {
        let a = assets::assets();
        let fonts = &a.fonts;

        clear_background(Color::from_rgba(10, 4, 12, 255));
        self.draw_backdrop(fonts);
        self.draw_title(fonts);

        if let Some(ov) = &self.overlay {
            ui::dim_overlay(0.65);
            match ov {
                MenuOverlay::Load => {
                    let res = overlay::save_load_panel(fonts, SaveMode::Load, None, &mut self.confirm_overwrite);
                    match res {
                        overlay::SaveLoadResult::None => {}
                        overlay::SaveLoadResult::Close => self.overlay = None,
                        overlay::SaveLoadResult::Load(i) => {
                            let data = save::load_into_state(i);
                            self.overlay = None;
                            return ScreenEvent::StartScene(Box::new(data));
                        }
                    }
                }
                MenuOverlay::Settings => {
                    match overlay::settings_panel(fonts) {
                        SettingsResult::None => {}
                        SettingsResult::Close => self.overlay = None,
                        SettingsResult::LangChanged => audio::play_sfx("menu_select"),
                    }
                }
            }
            if is_key_pressed(KeyCode::Escape) {
                self.overlay = None;
            }
            return ScreenEvent::None;
        }

        let bw = 300.0;
        let bh = 56.0;
        let bx = (W - bw) / 2.0;
        let mut by = 320.0;

        let new_clicked = ui::button(fonts, Rect::new(bx, by, bw, bh), lang::t("btn.newgame"), 24.0, ButtonStyle::default());
        by += bh + 16.0;
        let load_clicked = ui::button(fonts, Rect::new(bx, by, bw, bh), lang::t("btn.loadgame"), 24.0, ButtonStyle::default());
        by += bh + 16.0;
        let set_clicked = ui::button(fonts, Rect::new(bx, by, bw, bh), lang::t("btn.settings"), 24.0, ButtonStyle::default());
        by += bh + 16.0;
        let quit_clicked = ui::button(fonts, Rect::new(bx, by, bw, bh), lang::t("btn.quit"), 24.0, ButtonStyle::default());

        if new_clicked {
            audio::play_sfx("menu_select");
            return ScreenEvent::StartGame;
        }
        if load_clicked {
            audio::play_sfx("menu_select");
            self.overlay = Some(MenuOverlay::Load);
        }
        if set_clicked {
            audio::play_sfx("menu_select");
            self.overlay = Some(MenuOverlay::Settings);
        }
        if quit_clicked {
            return ScreenEvent::Quit;
        }

        ScreenEvent::None
    }

    fn draw_backdrop(&mut self, _fonts: &Fonts) {
        let a = assets::assets();
        if a.menu_frames.is_empty() {
            return;
        }
        // Advance through the GIF frames by accumulated delay.
        self.elapsed += get_frame_time() * 1000.0;
        let total: u32 = a.menu_frames.iter().map(|f| f.delay_ms).sum();
        if total == 0 {
            self.frame_idx = 0;
        } else {
            let mut acc = self.elapsed as u32 % total;
            for (i, f) in a.menu_frames.iter().enumerate() {
                if acc < f.delay_ms {
                    self.frame_idx = i;
                    break;
                }
                acc -= f.delay_ms;
            }
        }
        // Upload only the visible frame; the GIF plays at ~1 fps so a single
        // cached texture keeps VRAM flat regardless of how many frames exist.
        if self.bg_tex.is_none() || self.bg_tex_idx != self.frame_idx {
            self.bg_tex = Some(Texture2D::from_image(&a.menu_frames[self.frame_idx].img));
            if let Some(t) = &self.bg_tex {
                t.set_filter(FilterMode::Linear);
            }
            self.bg_tex_idx = self.frame_idx;
        }
        if let Some(tex) = &self.bg_tex {
            draw_texture_ex(
                tex,
                0.0,
                0.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(W, H)),
                    ..Default::default()
                },
            );
        }
    }

    fn draw_title(&self, fonts: &Fonts) {
        ui::draw_text_center(fonts, lang::t("title.game"), W / 2.0, 190.0, 60.0, Color::from_rgba(235, 235, 255, 255));
        ui::draw_text_center(fonts, lang::t("title.sub"), W / 2.0, 250.0, 26.0, Color::from_rgba(205, 165, 235, 255));
    }
}

// ── Name entry ─────────────────────────────────────────────────────────────

pub struct NameEntry {
    value: String,
}

impl NameEntry {
    pub fn new() -> NameEntry {
        NameEntry { value: String::new() }
    }

    /// The confirmed player name (falls back to a default when empty).
    pub fn player_name(&self) -> String {
        let name = self.value.trim().to_string();
        if name.is_empty() {
            lang::t("name.default").to_string()
        } else {
            name
        }
    }

    /// Returns true when the player confirmed the name.
    pub fn frame(&mut self) -> bool {
        let a = assets::assets();
        let fonts = &a.fonts;

        clear_background(Color::from_rgba(12, 4, 16, 255));
        ui::draw_text_center(fonts, lang::t("name.title"), W / 2.0, 180.0, 44.0, Color::from_rgba(235, 170, 225, 255));
        ui::draw_text_center(fonts, lang::t("name.prompt"), W / 2.0, 270.0, 24.0, Color::from_rgba(235, 230, 242, 255));

        // Text field
        let fw = 460.0;
        let fh = 56.0;
        let fx = (W - fw) / 2.0;
        let fy = 330.0;
        if let Some(c) = get_char_pressed() {
            if c.is_alphanumeric() && c != ' ' {
                if self.value.len() < 18 {
                    self.value.push(c);
                }
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.value.pop();
        }
        let rect = Rect::new(fx, fy, fw, fh);
        let focused = true;
        ui::draw_rounded_rect(rect.x, rect.y, rect.w, rect.h, 8.0, Color::from_rgba(30, 10, 36, 255));
        ui::draw_rounded_rect_border(rect.x, rect.y, rect.w, rect.h, 8.0, 2.0, Color::from_rgba(190, 90, 170, 255));
        let display = if self.value.is_empty() { lang::t("name.placeholder") } else { &self.value };
        let color = if self.value.is_empty() {
            Color::from_rgba(140, 110, 140, 255)
        } else {
            Color::from_rgba(255, 245, 255, 255)
        };
        ui::draw_text_center(fonts, display, W / 2.0, fy + fh / 2.0 + 9.0, 22.0, color);
        if focused {
            let blink = ((get_time() * 2.0) as i32) % 2 == 0;
            if blink && !self.value.is_empty() {
                let tw = ui::text_width(fonts, &self.value, 22.0);
                ui::draw_text(fonts, "|", W / 2.0 + tw / 2.0 + 4.0, fy + fh / 2.0 + 9.0, 22.0, Color::from_rgba(255, 170, 220, 255));
            }
        }

        let bw = 220.0;
        let bh = 50.0;
        let bx = (W - bw) / 2.0;
        let by = fy + fh + 50.0;
        let clicked = ui::button(fonts, Rect::new(bx, by, bw, bh), lang::t("name.confirm"), 22.0, ButtonStyle::default())
            || is_key_pressed(KeyCode::Enter);
        if clicked {
            state::player_mut().set_name(&self.player_name());
            audio::play_sfx("menu_select");
        }
        clicked
    }
}

// ── Act card ───────────────────────────────────────────────────────────────

pub struct ActCard {
    pub title: String,
    pub subtitle: String,
    pub next: String,
    elapsed: f32,
}

impl ActCard {
    pub fn new(title: &str, subtitle: &str, next: &str) -> ActCard {
        ActCard {
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            next: next.to_string(),
            elapsed: 0.0,
        }
    }

    /// Returns `Some(next_scene)` when the card has been shown (or skipped).
    pub fn frame(&mut self) -> Option<String> {
        self.elapsed += get_frame_time();
        clear_background(BLACK);
        let a = assets::assets();
        let fonts = &a.fonts;

        let alpha = match self.elapsed {
            t if t < 0.8 => t / 0.8,
            t if t < 3.6 => 1.0,
            t if t < 4.4 => 1.0 - (t - 3.6) / 0.8,
            _ => 0.0,
        };
        let color = Color::new(1.0, 0.72, 0.9, alpha.clamp(0.0, 1.0));
        let sub_color = Color::new(0.85, 0.8, 0.85, alpha.clamp(0.0, 1.0));

        ui::draw_text_center(fonts, &self.title, W / 2.0, 340.0, 52.0, color);
        ui::draw_text_italic(fonts, &self.subtitle, W / 2.0 - ui::text_width(fonts, &self.subtitle, 26.0) / 2.0, 405.0, 26.0, sub_color);

        if self.elapsed > 4.4
            || is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Space)
            || is_mouse_button_pressed(MouseButton::Left)
        {
            return Some(self.next.clone());
        }
        None
    }
}

// ── Credits ────────────────────────────────────────────────────────────────

pub struct Credits {
    scroll: f32,
}

const CREDIT_ROWS: &[&str] = &[
    "~ WORM WORM LITERATURE CLUB ~",
    "Praise be to Scatha.",
    "",
    "STORY & WRITING",
    "Foui_",
    "",
    "PROGRAMMING",
    "Foui_",
    "",
    "ART & CHARACTER DESIGN",
    "Foui_",
    "",
    "MUSIC",
    "Original Soundtrack",
    "",
    "SPECIAL THANKS",
    "Scatha, the Great Worm",
    "The Scatha cult",
    "SHNOFFI",
    "",
    "",
    "Thank you for playing.",
];

impl Credits {
    pub fn new() -> Credits {
        Credits { scroll: H + 60.0 }
    }

    /// Returns true when the credits finished (or were skipped).
    pub fn frame(&mut self) -> bool {
        self.scroll -= get_frame_time() * 60.0;
        clear_background(BLACK);
        let a = assets::assets();
        let fonts = &a.fonts;

        let mut y = self.scroll;
        for (i, row) in CREDIT_ROWS.iter().enumerate() {
            if row.is_empty() {
                y += 26.0;
                continue;
            }
            let title = i % 4 == 0 || row.starts_with("~");
            if title {
                ui::draw_text_center(fonts, row, W / 2.0, y, 26.0, Color::from_rgba(255, 136, 204, 255));
            } else {
                ui::draw_text_center(fonts, row, W / 2.0, y, 18.0, Color::from_rgba(235, 225, 238, 255));
            }
            y += 34.0;
        }

        ui::draw_text_center(fonts, lang::t("credits.skip"), W / 2.0, H - 24.0, 14.0, Color::from_rgba(150, 119, 170, 255));
        y < -40.0 || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Space)
    }
}
