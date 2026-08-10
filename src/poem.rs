//! The poem-writing minigame: build a 5-line poem by picking words.

use macroquad::prelude::*;

use crate::assets::{self, Fonts};
use crate::ui::{self, ButtonStyle, W};
use crate::{audio, lang, state};

const LEAD_INS: [&str; 5] = [
    "Below the surface of things, there is",
    "What moves through darkness does so",
    "I came to this place not knowing",
    "The worm does not ask if the earth",
    "And so I find that I have stayed",
];

const LEAD_INS_FR: [&str; 5] = [
    "Sous la surface des choses, il y a",
    "Ce qui se meut dans les ténèbres le fait",
    "Je suis venu ici sans savoir",
    "Le ver ne demande pas si la terre",
    "Et voilà que je suis resté",
];

const WORD_SETS: [[[&str; 3]; 5]; 4] = [
    // English
    [
        ["a garden", "a hole", "a heartbeat"],
        ["with purpose", "with hunger", "with patience"],
        ["what love was for", "what I would become", "why the door was open"],
        ["was worthy", "was willing", "was warm"],
        ["for the words", "for the dirt", "for the faces"],
    ],
    // French
    [
        ["un jardin", "un trou", "un battement de cœur"],
        ["avec intention", "avec faim", "avec patience"],
        ["ce qu'était l'amour", "ce que je deviendrais", "pourquoi la porte était ouverte"],
        ["était digne", "était consentante", "était chaude"],
        ["pour les mots", "pour la terre", "pour les visages"],
    ],
    // English (sardonic)
    [
        ["a joke", "a trap", "a test"],
        ["clumsily", "relentlessly", "quietly"],
        ["what it meant", "what I owed", "when it would end"],
        ["was ours", "was nothing", "was watching"],
        ["for the noise", "for the silence", "for the company"],
    ],
    // French (sardonic)
    [
        ["une farce", "un piège", "une épreuve"],
        ["maladroitement", "sans relâche", "en silence"],
        ["ce que cela voulait dire", "ce que je devais", "quand cela finirait"],
        ["était à nous", "n'était rien", "nous observait"],
        ["pour le bruit", "pour le silence", "pour la compagnie"],
    ],
];

struct WordChoice {
    lead_in: &'static str,
    options: [&'static str; 3],
}

pub struct Poem {
    lines: Vec<WordChoice>,
    picked: Vec<usize>,
    current: usize,
    revealing: bool,
    timer: f32,
    done: bool,
}

impl Poem {
    pub fn new() -> Poem {
        let fr = lang::is_french();
        let set_idx = if fr { 1 } else { 0 };
        let lead_ins = if fr { LEAD_INS_FR } else { LEAD_INS };
        let sets = WORD_SETS[set_idx];
        let lines = (0..5)
            .map(|i| WordChoice {
                lead_in: lead_ins[i],
                options: sets[i],
            })
            .collect();
        Poem {
            lines,
            picked: Vec::new(),
            current: 0,
            revealing: false,
            timer: 0.0,
            done: false,
        }
    }

    /// Returns true when the player has finished and continues.
    pub fn frame(&mut self) -> bool {
        let a = assets::assets();
        let fonts = &a.fonts;

        clear_background(Color::from_rgba(12, 4, 16, 255));
        ui::draw_text_center(fonts, lang::t("poem.title"), W / 2.0, 120.0, 38.0, Color::from_rgba(230, 160, 225, 255));

        if !self.revealing {
            self.draw_accumulated(fonts, 200.0);
            self.draw_current_line(fonts);
        } else {
            self.reveal(fonts);
        }

        self.done
    }

    fn draw_accumulated(&self, fonts: &Fonts, start_y: f32) {
        let mut y = start_y;
        for (i, &choice) in self.picked.iter().enumerate() {
            if i >= self.current && !self.done {
                break;
            }
            let line = &self.lines[i];
            let text = format!("{} {}", line.lead_in, line.options[choice]);
            ui::draw_text_center(fonts, &text, W / 2.0, y, 20.0, Color::from_rgba(225, 215, 230, 255));
            y += 34.0;
        }
    }

    fn draw_current_line(&mut self, fonts: &Fonts) {
        if self.current >= 5 {
            return;
        }
        let line = &self.lines[self.current];
        ui::draw_text_center(fonts, line.lead_in, W / 2.0, 470.0, 24.0, Color::from_rgba(245, 225, 245, 255));

        let bw = 300.0;
        let bh = 54.0;
        let gap = 24.0;
        let total = bw * 3.0 + gap * 2.0;
        let start_x = (W - total) / 2.0;
        let y = 520.0;

        for (i, word) in line.options.iter().enumerate() {
            let rect = Rect::new(start_x + i as f32 * (bw + gap), y, bw, bh);
            if ui::button(fonts, rect, word, 22.0, ButtonStyle::default()) {
                self.picked.push(i);
                self.current += 1;
                audio::play_sfx("menu_select");
                if self.current >= 5 {
                    self.revealing = true;
                    self.timer = 0.0;
                }
            }
        }
    }

    fn reveal(&mut self, fonts: &Fonts) {
        self.timer += get_frame_time();
        let visible = (self.timer / 0.35).floor() as usize;
        let mut y = 260.0;
        for i in 0..visible.min(5) {
            let line = &self.lines[i];
            let text = format!("{} {}", line.lead_in, line.options[self.picked[i]]);
            ui::draw_text_center(fonts, &text, W / 2.0, y, 20.0, Color::from_rgba(225, 215, 230, 255));
            y += 36.0;
        }
        if visible >= 5 {
            // Save the finished poem and its "tone" into player state.
            let mut poem_text = String::new();
            for (i, &choice) in self.picked.iter().enumerate() {
                let line = &self.lines[i];
                if !poem_text.is_empty() {
                    poem_text.push('\n');
                }
                poem_text.push_str(&format!("{} {}", line.lead_in, line.options[choice]));
            }
            let tone = dominant_tone(&self.picked, lang::is_french());
            state::player_mut().poem_tone = tone.to_string();
            state::player_mut().poem_text = poem_text;

            ui::draw_text_center(fonts, lang::t("poem.finished"), W / 2.0, 520.0, 22.0, Color::from_rgba(230, 170, 225, 255));
            let bw = 240.0;
            let bh = 50.0;
            let bx = (W - bw) / 2.0;
            let by = 560.0;
            if ui::button(fonts, Rect::new(bx, by, bw, bh), lang::t("poem.continue"), 22.0, ButtonStyle::default())
                || is_key_pressed(KeyCode::Enter)
            {
                self.done = true;
                audio::play_sfx("menu_select");
            }
        }
    }
}

fn dominant_tone(picked: &[usize], _fr: bool) -> &'static str {
    // A light heuristic: which column was chosen most often?
    let mut counts = [0i32; 3];
    for &i in picked {
        counts[i] += 1;
    }
    let tones = ["earnest", "sardonic", "tender"];
    let mut best = 0;
    for i in 1..3 {
        if counts[i] > counts[best] {
            best = i;
        }
    }
    tones[best]
}
