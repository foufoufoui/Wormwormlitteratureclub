//! The in-game screen: dialogue box, typewriter text, character sprites,
//! choices, HUD and the pause overlays (menu / save-load / settings).

use std::collections::HashMap;

use macroquad::prelude::*;

use crate::assets::{self, Fonts};
use crate::dialogue::{Choice, Cmd};
use crate::event::ScreenEvent;
use crate::overlay::{self, SaveLoadResult, SaveMode, SettingsResult};
use crate::ui::{self, ButtonStyle, W, H};
use crate::{audio, lang, save, settings, state};

/// Dialogue box logical coordinates (fixed 1280x720 canvas).
const BOX_X: f32 = 30.0;
const BOX_Y: f32 = 495.0;
const BOX_W: f32 = 1220.0;
const BOX_H: f32 = 195.0;

#[derive(Clone)]
pub struct CharView {
    pub name: String,
    pub expr: String,
    pub pos: String,
    pub visible: bool,
    pub alpha: f32,
    pub target_alpha: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    Confirm,
    SaveLoad(SaveMode),
    Settings,
}

pub struct Game {
    pub dialogue: crate::dialogue::Dialogue,
    pub bg: Option<Texture2D>,
    /// Key of the currently displayed background (for save snapshots).
    pub bg_key: Option<String>,
    pub chars: HashMap<String, CharView>,
    pub speaker: Option<String>,
    pub full_text: String,
    pub revealed: usize,
    pub text_done: bool,
    pub type_accum: f32,
    pub choices: Vec<Choice>,
    pub selected: usize,
    pub auto: bool,
    pub auto_timer: f32,
    pub skip_timer: f32,
    pub overlay: Overlay,
    pub confirm_overwrite: Option<usize>,
    pub player_name: String,
    pub pending: Option<ScreenEvent>,
    pub arrow_t: f32,
    /// Seconds remaining in a `wait` command (background-only pause).
    pub wait_timer: f32,
}

impl Game {
    pub fn new(player_name: &str, start_scene: &str) -> Game {
        let mut g = Game {
            dialogue: crate::dialogue::Dialogue::new(),
            bg: None,
            bg_key: None,
            chars: HashMap::new(),
            speaker: None,
            full_text: String::new(),
            revealed: 0,
            text_done: true,
            type_accum: 0.0,
            choices: Vec::new(),
            selected: 0,
            auto: false,
            auto_timer: 0.0,
            skip_timer: 0.0,
            overlay: Overlay::None,
            confirm_overwrite: None,
            player_name: player_name.to_string(),
            pending: None,
            arrow_t: 0.0,
            wait_timer: 0.0,
        };
        let cmds = g.dialogue.load_and_advance(start_scene);
        g.apply(cmds);
        g
    }

    /// Rebuild a game from a save snapshot: restores the dialogue position,
    /// background, characters, current line and music.
    pub fn from_save(data: &save::SaveData) -> Game {
        let mut g = Game::new(&data.player_name, &data.scene);
        let out = g.dialogue.resume(&data.scene, data.cursor);
        for c in out {
            if let Cmd::Choices(choices) = c {
                g.choices = choices;
                g.selected = 0;
            }
        }
        if let Some(bg) = &data.bg {
            g.bg_key = Some(bg.clone());
            g.bg = assets::assets().bg.get(bg).cloned();
        }
        g.chars = data
            .chars
            .iter()
            .map(|cs| {
                (
                    cs.name.clone(),
                    CharView {
                        name: cs.name.clone(),
                        expr: cs.expr.clone(),
                        pos: cs.pos.clone(),
                        visible: cs.visible,
                        alpha: if cs.visible { 1.0 } else { 0.0 },
                        target_alpha: if cs.visible { 1.0 } else { 0.0 },
                    },
                )
            })
            .collect();
        if let Some(music) = &data.music {
            audio::play_bgm(music);
        }
        g.speaker = data.speaker.clone();
        g.full_text = data.full_text.clone();
        g.revealed = data.revealed.min(g.full_text.chars().count());
        g.text_done = g.revealed >= g.full_text.chars().count();
        g
    }

    /// Snapshot the current visual + dialogue state for the save panel.
    pub fn snapshot(&self) -> save::SaveData {
        save::SaveData {
            scene: self.dialogue.scene().to_string(),
            cursor: self.dialogue.cursor(),
            bg: self.bg_key.clone(),
            music: audio::current_bgm(),
            chars: self
                .chars
                .values()
                .map(|cv| save::CharSnapshot {
                    name: cv.name.clone(),
                    expr: cv.expr.clone(),
                    pos: cv.pos.clone(),
                    visible: cv.visible,
                })
                .collect(),
            speaker: self.speaker.clone(),
            full_text: self.full_text.clone(),
            revealed: self.revealed,
            text_done: self.text_done,
            player_name: self.player_name.clone(),
            poem_tone: state::player().poem_tone.clone(),
            poem_text: state::player().poem_text.clone(),
            route_character: state::player().route_character.clone(),
            flags: state::player().flags.clone(),
            timestamp: String::new(),
        }
    }

    /// Apply a batch of engine events.
    fn apply(&mut self, cmds: Vec<Cmd>) {
        for c in cmds {
            match c {
                Cmd::Line { speaker, text } => self.show_line(speaker, &text),
                Cmd::Bg(file) => {
                    self.bg_key = Some(file.clone());
                    self.bg = assets::assets().bg.get(&file).cloned();
                }
                Cmd::Music(file) => audio::play_bgm(&file),
                Cmd::Sfx(file) => audio::play_sfx(&file),
                Cmd::Char { name, expr, pos, visible } => self.set_char(&name, &expr, &pos, visible),
                Cmd::Choices(choices) => {
                    self.choices = choices;
                    self.selected = 0;
                }
                Cmd::Wait(secs) => self.wait_timer = secs,
                Cmd::Poem => self.pending = Some(ScreenEvent::Poem),
                Cmd::End => self.pending = Some(ScreenEvent::Credits),
                Cmd::ActCard { title, subtitle, next } => {
                    self.pending = Some(ScreenEvent::ActCard(title, subtitle, next));
                }
            }
        }
        state::player_mut().current_scene = self.dialogue.scene().to_string();
    }

    fn show_line(&mut self, speaker: Option<String>, text: &str) {
        let text = text.replace("[player]", &self.player_name);
        let speaker = speaker.map(|s| if s == "[player]" { self.player_name.clone() } else { s });
        self.speaker = speaker;
        self.full_text = text;
        self.revealed = 0;
        self.text_done = self.full_text.is_empty();
        self.type_accum = 0.0;
        self.choices.clear();
    }

    fn set_char(&mut self, name: &str, expr: &str, pos: &str, visible: bool) {
        let cv = self
            .chars
            .entry(name.to_string())
            .or_insert_with(|| CharView {
                name: name.to_string(),
                expr: "normal".to_string(),
                pos: "center".to_string(),
                visible: false,
                alpha: 0.0,
                target_alpha: 0.0,
            });
        if visible {
            cv.name = name.to_string();
            cv.expr = expr.to_string();
            cv.pos = pos.to_string();
            cv.visible = true;
            cv.target_alpha = 1.0;
            if cv.alpha <= 0.0 {
                cv.alpha = 0.001;
            }
        } else {
            cv.visible = false;
            cv.target_alpha = 0.0;
        }
    }

    // ── Frame ──────────────────────────────────────────────────────────────

    /// Update and draw; returns a screen-transition event (consuming it).
    pub fn frame(&mut self) -> ScreenEvent {
        if let Some(ev) = self.pending.take() {
            return ev;
        }
        self.update();
        self.draw();
        ScreenEvent::None
    }

    fn update(&mut self) {
        let dt = get_frame_time();
        self.arrow_t += dt;

        // During a `wait` command only the background is shown: block all
        // other input and flow until the timer elapses (or the player skips).
        if self.wait_timer > 0.0 {
            self.update_wait(dt);
            return;
        }

        self.update_typewriter(dt);
        self.update_char_fades(dt);

        // Global keys
        if is_key_pressed(KeyCode::F11) || (is_key_pressed(KeyCode::F) && is_key_down(KeyCode::LeftControl)) {
            settings::toggle_fullscreen();
        }
        if is_key_pressed(KeyCode::Escape) {
            match self.overlay {
                Overlay::None => {
                    if settings::is_fullscreen() {
                        settings::toggle_fullscreen();
                    } else {
                        self.overlay = Overlay::Confirm;
                    }
                }
                _ => {
                    self.overlay = Overlay::None;
                    self.confirm_overwrite = None;
                }
            }
        }

        match self.overlay {
            Overlay::None => self.update_gameplay(),
            Overlay::Confirm | Overlay::SaveLoad(_) | Overlay::Settings => {}
        }
    }

    fn update_wait(&mut self, dt: f32) {
        if is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Space)
            || is_mouse_button_pressed(MouseButton::Left)
        {
            self.wait_timer = 0.0;
            self.try_advance();
            return;
        }
        self.wait_timer -= dt;
        if self.wait_timer <= 0.0 {
            self.wait_timer = 0.0;
            self.try_advance();
        }
    }

    fn update_gameplay(&mut self) {
        if is_key_pressed(KeyCode::A) {
            self.auto = !self.auto;
            self.auto_timer = 0.0;
        }

        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl {
            self.skip_timer += get_frame_time();
            if self.skip_timer > 0.06 {
                self.skip_timer = 0.0;
                self.try_advance();
            }
        }

        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            if !self.choices.is_empty() {
                let idx = self.selected;
                let cmds = self.dialogue.pick_choice(idx);
                self.apply(cmds);
            } else {
                self.try_advance();
            }
        }

        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Down) {
            self.navigate_choice();
        }

        // Auto-advance
        if self.auto && self.text_done && self.choices.is_empty() {
            self.auto_timer += get_frame_time();
            if self.auto_timer > 3.0 {
                self.auto_timer = 0.0;
                self.try_advance();
            }
        }
    }

    fn navigate_choice(&mut self) {
        if self.choices.len() < 2 {
            return;
        }
        if is_key_pressed(KeyCode::Up) {
            self.selected = (self.selected + self.choices.len() - 1) % self.choices.len();
        } else {
            self.selected = (self.selected + 1) % self.choices.len();
        }
        audio::play_sfx("menu_select");
    }

    fn try_advance(&mut self) {
        if self.dialogue.is_waiting() {
            return;
        }
        if !self.text_done {
            self.reveal_all();
        } else {
            let cmds = self.dialogue.advance();
            self.apply(cmds);
        }
    }

    fn reveal_all(&mut self) {
        self.revealed = self.full_text.chars().count();
        self.text_done = true;
    }

    fn update_typewriter(&mut self, dt: f32) {
        if self.text_done {
            return;
        }
        let delay = settings::settings().char_delay_ms() / 1000.0;
        if delay <= 0.0 {
            self.reveal_all();
            return;
        }
        self.type_accum += dt;
        let target = ((self.type_accum / delay) as usize).min(self.full_text.chars().count());
        while self.revealed < target {
            self.revealed += 1;
            audio::play_sfx("text_blip");
        }
        if self.revealed >= self.full_text.chars().count() {
            self.text_done = true;
        }
    }

    fn update_char_fades(&mut self, dt: f32) {
        let rate = 6.0;
        for cv in self.chars.values_mut() {
            let diff = cv.target_alpha - cv.alpha;
            if diff.abs() > 0.001 {
                let step = (diff.abs() * rate * dt).min(diff.abs());
                cv.alpha += step.signum() * diff.abs().min(step);
            }
            if !cv.visible && cv.alpha <= 0.01 {
                cv.alpha = 0.0;
            }
        }
    }

    // ── Drawing ────────────────────────────────────────────────────────────

    fn draw(&mut self) {
        let a = assets::assets();
        let fonts = &a.fonts;

        clear_background(Color::from_rgba(10, 4, 12, 255));
        self.draw_bg(self.bg.as_ref());
        // During a `wait` the stage is empty so the background can be admired.
        if self.wait_timer <= 0.0 {
            self.draw_chars(fonts);
            self.draw_gui(fonts);
        }

        match self.overlay {
            Overlay::None => {}
            Overlay::Confirm => self.draw_confirm(fonts),
            Overlay::SaveLoad(mode) => self.draw_save_load(fonts, mode),
            Overlay::Settings => self.draw_settings(fonts),
        }
    }

    fn draw_bg(&self, bg: Option<&Texture2D>) {
        if let Some(bg) = bg {
            draw_texture_ex(
                bg,
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

    fn draw_chars(&self, fonts: &Fonts) {
        let assets = assets::assets();
        for cv in self.chars.values() {
            if cv.alpha < 0.01 {
                continue;
            }
            let tint = Color::new(1.0, 1.0, 1.0, cv.alpha.clamp(0.0, 1.0));
            if let Some((tex, src)) = assets.char_frame(&cv.name, &cv.expr) {
                // Size the visible character art (src) to a fixed height,
                // keeping its native aspect ratio instead of squishing the
                // whole canvas into a square.
                let h = 0.63 * H;
                let w = h * (src.w / src.h);
                let (x, y, w, h) = char_rect(cv, w, h);
                draw_texture_ex(
                    &tex,
                    x,
                    y,
                    tint,
                    DrawTextureParams {
                        source: Some(src),
                        dest_size: Some(vec2(w, h)),
                        ..Default::default()
                    },
                );
            } else {
                // Placeholder for a character with no art yet.
                let color = char_color(&cv.name);
                let (x, y, w, h) = char_rect(cv, 0.63 * H, 0.63 * H);
                ui::draw_rounded_rect(x, y, w, h, 12.0, Color::new(color.r * 0.18, color.g * 0.18, color.b * 0.18, cv.alpha));
                ui::draw_text_center(fonts, &cv.name, x + w / 2.0, y + h / 2.0 - 6.0, 20.0, Color::new(color.r, color.g, color.b, cv.alpha));
            }
        }
    }

    fn draw_gui(&mut self, fonts: &Fonts) {
        if !self.choices.is_empty() {
            // While choices are shown the text box and HUD are hidden.
            self.draw_choices(fonts);
        } else {
            self.draw_box(fonts);
            self.draw_hud(fonts);
        }
    }

    fn draw_box(&mut self, fonts: &Fonts) {
        // Box backdrop
        ui::draw_rounded_rect(BOX_X, BOX_Y, BOX_W, BOX_H, 12.0, Color::new(0.04, 0.02, 0.05, 0.92));
        ui::draw_rounded_rect_border(BOX_X, BOX_Y, BOX_W, BOX_H, 12.0, 2.0, Color::from_rgba(120, 40, 90, 255));

        // Speaker plate: shown only for spoken dialogue, never for narration,
        // and flush with the box top so it can't overlap sprites or the text.
        let speaker = self.speaker.clone().unwrap_or_default();
        if !speaker.is_empty() {
            let plate_color = char_color(&speaker);
            let plate_w = 210.0;
            let plate_h = 38.0;
            ui::draw_rounded_rect(BOX_X, BOX_Y - plate_h, plate_w, plate_h, 8.0, Color::from_rgba(25, 8, 30, 255));
            ui::draw_rounded_rect_border(BOX_X, BOX_Y - plate_h, plate_w, plate_h, 8.0, 2.0, plate_color);
            ui::draw_text_bold(fonts, &speaker, BOX_X + 16.0, BOX_Y - plate_h + 25.0, 20.0, plate_color);
        }

        // Typewriter text
        let prefix: String = self.full_text.chars().take(self.revealed).collect();
        ui::draw_wrapped_text(fonts, &prefix, BOX_X + 24.0, BOX_Y + 26.0, 24.0, BOX_W - 48.0, Color::from_rgba(240, 235, 242, 255));

        // Blinking continue arrow
        if self.text_done && !self.dialogue.is_waiting() {
            let blink = 0.35 + 0.65 * (0.5 + 0.5 * (self.arrow_t * 3.0).sin());
            ui::draw_text_bold(fonts, ">", BOX_X + BOX_W - 34.0, BOX_Y + BOX_H - 14.0, 20.0, Color::new(0.9, 0.65, 0.85, blink));
        }
    }

    fn draw_choices(&mut self, fonts: &Fonts) {
        // Fullscreen dim layer, then the choices centered on screen.
        ui::dim_overlay(0.82);

        let cw = 760.0;
        let ch = 56.0;
        let gap = 18.0;
        let total = self.choices.len() as f32 * ch + (self.choices.len() - 1) as f32 * gap;
        let start_y = (H - total) / 2.0;
        let x = (W - cw) / 2.0;

        let mut clicked: Option<usize> = None;
        for (i, choice) in self.choices.iter().enumerate() {
            let y = start_y + i as f32 * (ch + gap);
            let style = if i == self.selected {
                ButtonStyle {
                    base: Color::from_rgba(140, 30, 96, 220),
                    hover: Color::from_rgba(170, 40, 120, 235),
                    border: Color::from_rgba(255, 140, 200, 255),
                    border_hover: Color::from_rgba(255, 180, 220, 255),
                    text: Color::from_rgba(255, 245, 255, 255),
                    text_hover: WHITE,
                }
            } else {
                ButtonStyle {
                    base: Color::from_rgba(40, 12, 48, 200),
                    hover: Color::from_rgba(70, 20, 70, 220),
                    border: Color::from_rgba(130, 60, 120, 180),
                    border_hover: Color::from_rgba(180, 90, 160, 220),
                    text: Color::from_rgba(225, 200, 225, 255),
                    text_hover: Color::from_rgba(255, 230, 255, 255),
                }
            };
            let label = choice.label.replace("[player]", &self.player_name);
            if ui::button(fonts, Rect::new(x, y, cw, ch), &label, 22.0, style) {
                clicked = Some(i);
            }
        }

        if let Some(i) = clicked {
            audio::play_sfx("menu_select");
            let cmds = self.dialogue.pick_choice(i);
            self.apply(cmds);
        }
    }

    fn draw_hud(&mut self, fonts: &Fonts) {
        let y = BOX_Y + BOX_H - 40.0;
        let btn_h = 34.0;
        let btn_w = 80.0;
        let x_right = BOX_X + BOX_W - btn_w - 14.0;
        let color = Color::from_rgba(230, 215, 230, 255);
        let hover = Color::from_rgba(255, 170, 220, 255);

        // Right-aligned from the right edge
        let mut x = x_right;
        if ui::text_button(fonts, Rect::new(x, y, btn_w, btn_h), lang::t("btn.settings"), 17.0, color, hover) {
            self.overlay = Overlay::Settings;
        }
        x -= btn_w + 8.0;
        if ui::text_button(fonts, Rect::new(x, y, btn_w, btn_h), lang::t("btn.save"), 17.0, color, hover) {
            self.overlay = Overlay::SaveLoad(SaveMode::Save);
            self.confirm_overwrite = None;
        }
        x -= btn_w + 8.0;
        let auto_color = if self.auto { Color::from_rgba(120, 255, 150, 255) } else { color };
        if ui::text_button(fonts, Rect::new(x, y, btn_w, btn_h), lang::t("btn.auto"), 17.0, auto_color, hover) {
            self.auto = !self.auto;
            self.auto_timer = 0.0;
        }
        x -= btn_w + 8.0;
        if ui::text_button(fonts, Rect::new(x, y, btn_w, btn_h), lang::t("btn.menu"), 17.0, color, hover) {
            self.overlay = Overlay::Confirm;
        }
    }

    // ── Overlays ───────────────────────────────────────────────────────────

    fn draw_confirm(&mut self, fonts: &Fonts) {
        ui::dim_overlay(0.65);
        // Size the box to the confirm_bg image's native aspect (880x370) so
        // the art isn't stretched.
        let w = 660.0;
        let h = w * 370.0 / 880.0;
        let x = (W - w) / 2.0;
        let y = (H - h) / 2.0;
        let a = assets::assets();
        let texture = a.confirm_bg.clone();
        if let Some(tex) = texture {
            draw_texture_ex(
                &tex,
                x,
                y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(w, h)),
                    ..Default::default()
                },
            );
            ui::draw_rounded_rect(x, y, w, h, 12.0, Color::new(0.06, 0.03, 0.07, 0.35));
        } else {
            ui::draw_rounded_rect(x, y, w, h, 12.0, Color::new(0.07, 0.03, 0.08, 0.97));
        }
        ui::draw_rounded_rect_border(x, y, w, h, 12.0, 2.0, Color::from_rgba(200, 90, 170, 255));

        // Multi-line so "Return to main menu?\nUnsaved progress..." wraps and
        // centres as a block instead of drawing a literal newline.
        ui::draw_multiline_center(fonts, lang::t("menu.confirm"), W / 2.0, y + h * 0.42, 26.0, Color::from_rgba(255, 240, 252, 255), 520.0);

        let bw = 170.0;
        let bh = 46.0;
        let bx = (W - bw) / 2.0;
        let by = y + h - 76.0;
        let style = ButtonStyle {
            base: Color::from_rgba(120, 30, 90, 220),
            hover: Color::from_rgba(150, 40, 115, 235),
            border: Color::from_rgba(230, 130, 190, 255),
            border_hover: Color::from_rgba(255, 170, 220, 255),
            text: Color::from_rgba(255, 245, 252, 255),
            text_hover: WHITE,
        };
        let leave_clicked = ui::button(fonts, Rect::new(bx - bw / 2.0 - 12.0, by, bw, bh), lang::t("confirm.leave"), 20.0, style);
        let stay_clicked = ui::button(fonts, Rect::new(bx + bw / 2.0 + 12.0, by, bw, bh), lang::t("confirm.stay"), 20.0, style);
        if leave_clicked {
            audio::stop_bgm();
            self.pending = Some(ScreenEvent::ToMenu);
            self.overlay = Overlay::None;
        }
        if stay_clicked {
            self.overlay = Overlay::None;
            self.confirm_overwrite = None;
        }
    }

    fn draw_save_load(&mut self, fonts: &Fonts, mode: SaveMode) {
        let snapshot = self.snapshot();
        let result = overlay::save_load_panel(fonts, mode, Some(&snapshot), &mut self.confirm_overwrite);
        match result {
            SaveLoadResult::None => {}
            SaveLoadResult::Close => {
                self.overlay = Overlay::None;
                self.confirm_overwrite = None;
            }
            SaveLoadResult::Load(i) => {
                let data = save::load_into_state(i);
                audio::stop_bgm();
                self.pending = Some(ScreenEvent::StartScene(Box::new(data)));
            }
        }
    }

    fn draw_settings(&mut self, fonts: &Fonts) {
        let result = overlay::settings_panel(fonts);
        match result {
            SettingsResult::None => {}
            SettingsResult::Close => {
                self.overlay = Overlay::None;
            }
            SettingsResult::LangChanged => {
                let cmds = self.dialogue.reload_current_scene();
                self.apply(cmds);
                audio::play_sfx("menu_select");
            }
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Place a sprite of size `w`x`h` bottom-aligned, anchored by its horizontal
/// center at the character's stage position.
fn char_rect(cv: &CharView, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let center_x = match cv.pos.as_str() {
        "left" => 210.0,
        "right" => 1070.0,
        _ => W / 2.0,
    };
    (center_x - w / 2.0, H - h, w, h)
}

pub fn char_color(name: &str) -> Color {
    match name {
        "Vermia" => Color::from_rgba(235, 90, 140, 255),
        "Grubwin" => Color::from_rgba(90, 150, 235, 255),
        "Mawla" => Color::from_rgba(90, 190, 140, 255),
        "Dirtus" => Color::from_rgba(220, 170, 90, 255),
        _ => Color::from_rgba(220, 120, 190, 255),
    }
}
