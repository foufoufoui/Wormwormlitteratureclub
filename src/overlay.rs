//! Reusable overlay panels for the settings and save/load screens, shared by
//! the main menu and the in-game HUD.

use macroquad::prelude::*;

use crate::assets::Fonts;
use crate::lang;
use crate::save;
use crate::settings;
use crate::ui::{self, ButtonStyle};

#[derive(Clone, Copy, PartialEq)]
pub enum SaveMode {
    Save,
    Load,
}

#[derive(Debug, PartialEq)]
pub enum SettingsResult {
    None,
    Close,
    LangChanged,
}

#[derive(Debug, PartialEq)]
pub enum SaveLoadResult {
    None,
    Close,
    Load(usize),
}

const PANEL_W: f32 = 600.0;
const PANEL_H: f32 = 470.0;

fn panel_rect() -> Rect {
    Rect::new(
        (ui::W - PANEL_W) / 2.0,
        (ui::H - PANEL_H) / 2.0,
        PANEL_W,
        PANEL_H,
    )
}

// ── Settings panel ────────────────────────────────────────────────────────

pub fn settings_panel(fonts: &Fonts) -> SettingsResult {
    let rect = panel_rect();
    ui::draw_rounded_rect(rect.x, rect.y, rect.w, rect.h, 14.0, Color::from_rgba(26, 5, 24, 255));
    ui::draw_rounded_rect_border(rect.x, rect.y, rect.w, rect.h, 14.0, 2.0, Color::from_rgba(204, 85, 170, 255));

    let cx = rect.x + rect.w / 2.0;
    let mut y = rect.y + 52.0;
    ui::draw_text_bold(fonts, lang::t("settings.title"), cx - 60.0, y, 34.0, Color::from_rgba(255, 136, 204, 255));
    y += 40.0;

    let mut result = SettingsResult::None;
    let s = settings::settings();

    // Text speed
    ui::draw_text_bold(fonts, lang::t("settings.speed"), rect.x + 60.0, y, 19.0, Color::from_rgba(240, 160, 204, 255));
    y += 26.0;
    let track = Rect::new(rect.x + 150.0, y, 300.0, 24.0);
    let speed = s.text_speed / 3.0;
    let new_speed = ui::slider(fonts, track, speed, Color::from_rgba(238, 102, 170, 255), Color::from_rgba(58, 8, 48, 255));
    if (new_speed - speed).abs() > 0.001 {
        settings::set_text_speed(new_speed * 3.0);
    }
    ui::draw_text(fonts, lang::t("settings.fast"), rect.x + 60.0, y + 6.0, 13.0, Color::from_rgba(204, 136, 170, 255));
    ui::draw_text(fonts, lang::t("settings.slow"), rect.x + rect.w - 90.0, y + 6.0, 13.0, Color::from_rgba(204, 136, 170, 255));
    y += 44.0;

    // Music volume
    ui::draw_text_bold(fonts, lang::t("settings.music"), rect.x + 60.0, y, 19.0, Color::from_rgba(240, 160, 204, 255));
    y += 26.0;
    let track = Rect::new(rect.x + 150.0, y, 300.0, 24.0);
    let music = s.music_volume;
    let new_music = ui::slider(fonts, track, music, Color::from_rgba(238, 102, 170, 255), Color::from_rgba(58, 8, 48, 255));
    if (new_music - music).abs() > 0.001 {
        settings::set_music_volume(new_music);
        crate::audio::set_bgm_volume(new_music);
    }
    ui::draw_text(fonts, lang::t("settings.off"), rect.x + 60.0, y + 6.0, 13.0, Color::from_rgba(204, 136, 170, 255));
    ui::draw_text(fonts, lang::t("settings.max"), rect.x + rect.w - 90.0, y + 6.0, 13.0, Color::from_rgba(204, 136, 170, 255));
    y += 44.0;

    // SFX volume
    ui::draw_text_bold(fonts, lang::t("settings.sfx"), rect.x + 60.0, y, 19.0, Color::from_rgba(240, 160, 204, 255));
    y += 26.0;
    let track = Rect::new(rect.x + 150.0, y, 300.0, 24.0);
    let sfx = s.sfx_volume;
    let new_sfx = ui::slider(fonts, track, sfx, Color::from_rgba(238, 102, 170, 255), Color::from_rgba(58, 8, 48, 255));
    if (new_sfx - sfx).abs() > 0.001 {
        settings::set_sfx_volume(new_sfx);
    }
    y += 48.0;

    // Language
    ui::draw_text_bold(fonts, lang::t("settings.lang"), rect.x + 60.0, y, 19.0, Color::from_rgba(240, 160, 204, 255));
    let fr = lang::is_french();
    let en_btn = Rect::new(cx - 130.0, y - 6.0, 120.0, 38.0);
    let fr_btn = Rect::new(cx + 10.0, y - 6.0, 120.0, 38.0);
    let mut active = ButtonStyle::default();
    active.base = Color::from_rgba(204, 85, 170, 255);
    active.hover = Color::from_rgba(204, 85, 170, 255);
    let mut inactive = ButtonStyle::default();
    inactive.base = Color::from_rgba(42, 5, 32, 255);
    inactive.hover = Color::from_rgba(42, 5, 32, 255);
    inactive.border = Color::from_rgba(122, 37, 85, 255);
    inactive.border_hover = Color::from_rgba(122, 37, 85, 255);
    inactive.text = Color::from_rgba(255, 170, 221, 255);
    inactive.text_hover = Color::from_rgba(255, 170, 221, 255);
    let (en_style, fr_style) = if fr { (inactive, active) } else { (active, inactive) };
    if ui::button(fonts, en_btn, "English", 16.0, en_style) && fr {
        settings::set_lang("en");
        result = SettingsResult::LangChanged;
    }
    if ui::button(fonts, fr_btn, "Français", 16.0, fr_style) && !fr {
        settings::set_lang("fr");
        result = SettingsResult::LangChanged;
    }
    y += 60.0;

    // Fullscreen + Close
    let fullscreen_label = if settings::is_fullscreen() {
        lang::t("btn.windowed")
    } else {
        lang::t("btn.fullscreen")
    };
    if ui::button(
        fonts,
        Rect::new(cx - 210.0, y, 200.0, 42.0),
        fullscreen_label,
        17.0,
        ButtonStyle::default(),
    ) {
        settings::toggle_fullscreen();
    }
    if ui::button(
        fonts,
        Rect::new(cx + 10.0, y, 200.0, 42.0),
        lang::t("btn.close"),
        17.0,
        ButtonStyle::default(),
    ) {
        result = SettingsResult::Close;
    }

    result
}

// ── Save / Load panel ─────────────────────────────────────────────────────

pub fn save_load_panel(
    fonts: &Fonts,
    mode: SaveMode,
    snapshot: Option<&save::SaveData>,
    confirm: &mut Option<usize>,
) -> SaveLoadResult {
    let rect = panel_rect();
    ui::draw_rounded_rect(rect.x, rect.y, rect.w, rect.h, 14.0, Color::from_rgba(26, 5, 24, 255));
    ui::draw_rounded_rect_border(rect.x, rect.y, rect.w, rect.h, 14.0, 2.0, Color::from_rgba(204, 85, 170, 255));

    let cx = rect.x + rect.w / 2.0;
    ui::draw_text_bold(
        fonts,
        if mode == SaveMode::Save { lang::t("save.title") } else { lang::t("load.title") },
        cx - 90.0,
        rect.y + 52.0,
        34.0,
        Color::from_rgba(255, 136, 204, 255),
    );

    let mut result = SaveLoadResult::None;

    // If an overwrite confirmation is pending, draw it and stop.
    if let Some(slot_idx) = *confirm {
        let box_rect = Rect::new(cx - 180.0, rect.y + 150.0, 360.0, 140.0);
        ui::draw_rounded_rect(box_rect.x, box_rect.y, box_rect.w, box_rect.h, 10.0, Color::from_rgba(13, 0, 16, 255));
        ui::draw_rounded_rect_border(box_rect.x, box_rect.y, box_rect.w, box_rect.h, 10.0, 2.0, Color::from_rgba(204, 85, 170, 255));
        ui::draw_multiline_center(fonts, lang::t("save.overwrite"), cx, box_rect.y + 40.0, 18.0, Color::from_rgba(255, 136, 204, 255), 320.0);
        if ui::button(
            fonts,
            Rect::new(box_rect.x + 20.0, box_rect.y + 90.0, 150.0, 38.0),
            lang::t("save.overwrite_btn"),
            16.0,
            ButtonStyle::default(),
        ) {
            if let Some(data) = snapshot {
                save::save(slot_idx, data);
                crate::audio::play_sfx("menu_select");
            }
            *confirm = None;
        }
        if ui::button(
            fonts,
            Rect::new(box_rect.x + 190.0, box_rect.y + 90.0, 150.0, 38.0),
            lang::t("save.cancel"),
            16.0,
            ButtonStyle::default(),
        ) {
            *confirm = None;
        }
        return result;
    }

    let slots = save::load_all();
    let mut y = rect.y + 100.0;
    for (i, slot) in slots.iter().enumerate() {
        let row = Rect::new(rect.x + 40.0, y, rect.w - 80.0, 74.0);
        ui::draw_rounded_rect(row.x, row.y, row.w, row.h, 8.0, Color::from_rgba(42, 5, 32, 255));
        ui::draw_rounded_rect_border(row.x, row.y, row.w, row.h, 8.0, 1.0, Color::new(0.78, 0.31, 0.63, 0.35));

        let label = if slot.is_empty() {
            format!("{} {}", lang::t("save.slot"), i + 1)
        } else {
            format!("{} {} — {}", lang::t("save.slot"), i + 1, slot.player_name)
        };
        ui::draw_text_bold(fonts, &label, row.x + 18.0, row.y + 24.0, 16.0, Color::from_rgba(255, 136, 204, 255));

        let detail = if slot.is_empty() {
            lang::t("save.empty").to_string()
        } else {
            format!("{}\n{}", save::display_name(&slot.scene), slot.timestamp)
        };
        ui::draw_text(fonts, &detail, row.x + 18.0, row.y + 48.0, 13.0, Color::from_rgba(221, 187, 221, 255));

        let action_rect = Rect::new(row.x + row.w - 92.0, row.y + 18.0, 76.0, 38.0);
        let enabled = mode == SaveMode::Save || !slot.is_empty();
        let action_label = match mode {
            SaveMode::Save => lang::t("save.save_btn").to_string(),
            SaveMode::Load => {
                if slot.is_empty() {
                    "—".to_string()
                } else {
                    lang::t("save.load_btn").to_string()
                }
            }
        };
        let mut style = ButtonStyle::default();
        if !enabled {
            style.base = Color::from_rgba(42, 5, 32, 60);
            style.hover = Color::from_rgba(42, 5, 32, 60);
        }
        if enabled && ui::button(fonts, action_rect, &action_label, 15.0, style) {
            if mode == SaveMode::Load {
                result = SaveLoadResult::Load(i);
                break;
            } else if slot.is_empty() {
                if let Some(data) = snapshot {
                    save::save(i, data);
                    crate::audio::play_sfx("menu_select");
                }
            } else {
                *confirm = Some(i);
            }
        }

        if !slot.is_empty() {
            let del_rect = Rect::new(row.x + row.w - 176.0, row.y + 18.0, 44.0, 38.0);
            if ui::button(fonts, del_rect, lang::t("save.delete"), 15.0, style) {
                save::delete(i);
                crate::audio::play_sfx("menu_select");
            }
        }

        y += 84.0;
    }

    if ui::button(
        fonts,
        Rect::new(cx - 90.0, rect.y + rect.h - 58.0, 180.0, 42.0),
        lang::t("btn.close"),
        17.0,
        ButtonStyle::default(),
    ) {
        result = SaveLoadResult::Close;
    }

    result
}
