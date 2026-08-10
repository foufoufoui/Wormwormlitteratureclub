//! Drawing + input helpers: text, wrapping, rounded rectangles, buttons,
//! sliders and a minimal text field.

use macroquad::prelude::*;

use crate::assets::Fonts;

pub const W: f32 = 1280.0;
pub const H: f32 = 720.0;

// ── Text ──────────────────────────────────────────────────────────────────

pub fn line_height(size: f32) -> f32 {
    size * 1.4
}

pub fn text_width(fonts: &Fonts, text: &str, size: f32) -> f32 {
    measure_text(text, Some(&fonts.regular), size as u16, 1.0).width
}

/// Word-wrap `text` to fit `max_width` at the given font size.
pub fn wrap_text(fonts: &Fonts, text: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w = 0.0f32;
    for word in text.split_whitespace() {
        let space_w = if current.is_empty() { 0.0 } else { text_width(fonts, " ", size) };
        let word_w = text_width(fonts, word, size);
        if current_w + space_w + word_w > max_width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_w = 0.0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_w += space_w;
        }
        current.push_str(word);
        current_w += word_w;
    }
    lines.push(current);
    lines
}

pub fn draw_text(fonts: &Fonts, text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(&fonts.regular),
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}

pub fn draw_text_bold(fonts: &Fonts, text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(&fonts.bold),
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}

pub fn draw_text_italic(fonts: &Fonts, text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(&fonts.italic),
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}

/// Draw centered text; returns the y position just below the block.
pub fn draw_text_center(fonts: &Fonts, text: &str, cx: f32, y: f32, size: f32, color: Color) {
    let w = text_width(fonts, text, size);
    draw_text(fonts, text, cx - w / 2.0, y, size, color);
}

/// Draw multi-line centered text (used for dialog messages).
pub fn draw_multiline_center(fonts: &Fonts, text: &str, cx: f32, y: f32, size: f32, color: Color, max_width: f32) {
    let lines = wrap_text(fonts, text, size, max_width);
    let lh = line_height(size);
    let total = lh * lines.len() as f32;
    let mut yy = y - total / 2.0 + lh;
    for line in lines {
        draw_text_center(fonts, &line, cx, yy, size, color);
        yy += lh;
    }
}

/// Word-wrapped paragraph of dialogue text.
pub fn draw_wrapped_text(fonts: &Fonts, text: &str, x: f32, y: f32, size: f32, max_width: f32, color: Color) {
    let lines = wrap_text(fonts, text, size, max_width);
    let lh = line_height(size);
    let mut yy = y;
    for line in lines {
        draw_text(fonts, &line, x, yy, size, color);
        yy += lh;
    }
}

// ── Shapes ────────────────────────────────────────────────────────────────

pub fn draw_rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let r = r.min(w / 2.0).min(h / 2.0);
    draw_rectangle(x + r, y, w - r * 2.0, h, color);
    draw_rectangle(x, y + r, r, h - r * 2.0, color);
    draw_rectangle(x + w - r, y + r, r, h - r * 2.0, color);
    draw_circle(x + r, y + r, r, color);
    draw_circle(x + w - r, y + r, r, color);
    draw_circle(x + r, y + h - r, r, color);
    draw_circle(x + w - r, y + h - r, r, color);
}

pub fn draw_rounded_rect_border(x: f32, y: f32, w: f32, h: f32, r: f32, thickness: f32, color: Color) {
    let r = r.min(w / 2.0).min(h / 2.0);
    draw_line(x + r, y, x + w - r, y, thickness, color);
    draw_line(x + r, y + h, x + w - r, y + h, thickness, color);
    draw_line(x, y + r, x, y + h - r, thickness, color);
    draw_line(x + w, y + r, x + w, y + h - r, thickness, color);
    let tr = r.min(thickness / 2.0);
    draw_circle(x + r, y + r, tr, color);
    draw_circle(x + w - r, y + r, tr, color);
    draw_circle(x + r, y + h - r, tr, color);
    draw_circle(x + w - r, y + h - r, tr, color);
}

/// Dim the whole screen behind an overlay.
pub fn dim_overlay(alpha: f32) {
    draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, alpha));
}

// ── Mouse / hit testing ───────────────────────────────────────────────────

pub fn mouse_pos() -> Vec2 {
    let (mx, my) = mouse_position();
    // Inverse of the letterbox transform used when presenting the canvas:
    // (virtual) = (window - offset) / scale.
    let scale = (screen_width() / W).min(screen_height() / H).max(0.0001);
    let ox = (screen_width() - W * scale) * 0.5;
    let oy = (screen_height() - H * scale) * 0.5;
    Vec2::new((mx - ox) / scale, (my - oy) / scale)
}

pub fn rect_contains(r: Rect, p: Vec2) -> bool {
    p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h
}

// ── Button ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct ButtonStyle {
    pub base: Color,
    pub hover: Color,
    pub border: Color,
    pub border_hover: Color,
    pub text: Color,
    pub text_hover: Color,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            base: Color::from_rgba(42, 5, 32, 235),
            hover: Color::from_rgba(100, 20, 70, 220),
            border: Color::new(1.0, 0.39, 0.71, 0.4),
            border_hover: Color::new(1.0, 0.59, 0.78, 0.9),
            text: Color::from_rgba(255, 255, 255, 255),
            text_hover: Color::from_rgba(255, 221, 238, 255),
        }
    }
}

/// Draws a button; returns true on click.
pub fn button(fonts: &Fonts, rect: Rect, label: &str, size: f32, style: ButtonStyle) -> bool {
    let p = mouse_pos();
    let hover = rect_contains(rect, p);
    let clicked = hover && is_mouse_button_pressed(MouseButton::Left);

    let bg = if hover { style.hover } else { style.base };
    let border = if hover { style.border_hover } else { style.border };
    let text = if hover { style.text_hover } else { style.text };
    draw_rounded_rect(rect.x, rect.y, rect.w, rect.h, 8.0, bg);
    draw_rounded_rect_border(rect.x, rect.y, rect.w, rect.h, 8.0, 1.5, border);
    let tw = text_width(fonts, label, size);
    draw_text(fonts, label, rect.x + (rect.w - tw) / 2.0, rect.y + (rect.h + size * 0.35) / 2.0, size, text);
    clicked
}

/// Borderless text-only button (used by the HUD); returns true on click.
pub fn text_button(fonts: &Fonts, rect: Rect, label: &str, size: f32, color: Color, hover_color: Color) -> bool {
    let p = mouse_pos();
    let hover = rect_contains(rect, p);
    let clicked = hover && is_mouse_button_pressed(MouseButton::Left);
    let tw = text_width(fonts, label, size);
    let c = if hover { hover_color } else { color };
    draw_text(fonts, label, rect.x, rect.y + (rect.h + size * 0.35) / 2.0, size, c);
    let _ = tw;
    clicked
}

// ── Slider ────────────────────────────────────────────────────────────────

/// Horizontal slider, 0..=1. Returns the (possibly changed) value.
pub fn slider(_fonts: &Fonts, rect: Rect, value: f32, accent: Color, track: Color) -> f32 {
    let p = mouse_pos();
    let hover = rect_contains(rect, p);
    let track_y = rect.y + rect.h / 2.0;
    draw_rounded_rect(rect.x, track_y - 3.0, rect.w, 6.0, 3.0, track);
    let knob_x = rect.x + value.clamp(0.0, 1.0) * rect.w;
    draw_circle(knob_x, track_y, 9.0, accent);
    draw_rounded_rect_border(rect.x, track_y - 3.0, rect.w, 6.0, 3.0, 1.0, accent);

    if (hover || is_mouse_button_down(MouseButton::Left)) && is_mouse_button_down(MouseButton::Left) {
        if rect_contains(Rect::new(rect.x, rect.y, rect.w, rect.h), p) {
            let t = ((p.x - rect.x) / rect.w).clamp(0.0, 1.0);
            return t;
        }
    }
    value
}
