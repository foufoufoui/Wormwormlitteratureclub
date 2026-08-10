//! Script engine. Reads story files from `assets/scripts/<name>.json` and
//! emits a stream of `Cmd` events for the game screen to apply.
//!
//! Beyond the original Java command set this also supports two new commands
//! that make the previously-unfinished route/flag system work:
//!
//! ```json
//! { "type": "set", "flag": "ritual", "value": "no" }
//! { "type": "if", "flag": "potato", "equals": "true", "jump": "act4_ending_potato" }
//! ```
//!
//! Choice options may also carry an optional `"set"` map applied on pick:
//! ```json
//! { "label": "...", "jump": "act1_scene1b_potato", "set": { "potato": "true" } }
//! ```

use std::collections::HashMap;

use serde_json::Value;

use crate::lang;
use crate::state;

#[derive(Debug, Clone)]
pub struct Choice {
    pub label: String,
    pub jump: String,
    pub set: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Line { speaker: Option<String>, text: String },
    Bg(String),
    Music(String),
    Sfx(String),
    Char { name: String, expr: String, pos: String, visible: bool },
    Choices(Vec<Choice>),
    /// Pause the story for `f32` seconds showing only the background (no
    /// characters, text box or HUD), then continue with the next command.
    Wait(f32),
    Poem,
    End,
    ActCard { title: String, subtitle: String, next: String },
}

pub struct Dialogue {
    scene: String,
    script: Vec<Value>,
    cursor: usize,
    waiting_for_choice: bool,
}

impl Dialogue {
    pub fn new() -> Self {
        Self {
            scene: String::new(),
            script: Vec::new(),
            cursor: 0,
            waiting_for_choice: false,
        }
    }

    pub fn scene(&self) -> &str {
        &self.scene
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting_for_choice
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn load_scene(&mut self, name: &str) {
        self.scene = name.to_string();
        let local = lang::script_name(name);
        self.script = read_script(&local);
        self.cursor = 0;
        self.waiting_for_choice = false;
    }

    /// Load a scene and immediately advance to its first display command.
    pub fn load_and_advance(&mut self, name: &str) -> Vec<Cmd> {
        self.load_scene(name);
        self.advance()
    }

    /// Advance until the engine stops (a `line`/`choice`/`poem`/`actcard`/
    /// `end` command). Auto-applied commands (`bg`, `music`, `char`, `set`,
    /// `jump`, `if`, ...) are collapsed into the returned event list.
    pub fn advance(&mut self) -> Vec<Cmd> {
        let mut out = Vec::new();
        self.advance_into(&mut out);
        out
    }

    fn advance_into(&mut self, out: &mut Vec<Cmd>) {
        if self.waiting_for_choice {
            return;
        }
        if self.cursor >= self.script.len() {
            out.push(Cmd::End);
            return;
        }
        let cmd = self.script[self.cursor].clone();
        self.cursor += 1;
        if self.process(&cmd, true, out) {
            self.advance_into(out);
        }
    }

    /// Reload the current scene in the current language and re-run every
    /// command already executed so the visuals are rebuilt *without* skipping
    /// forward. Fixes the Java bug where changing language restarted the
    /// whole scene (losing your place). Returns the events to re-apply.
    pub fn reload_current_scene(&mut self) -> Vec<Cmd> {
        let target = self.cursor;
        let scene = self.scene.clone();
        self.load_scene(&scene);
        let mut out = Vec::new();
        while self.cursor < target && !self.waiting_for_choice {
            let cmd = self.script[self.cursor].clone();
            self.cursor += 1;
            if is_wait_cmd(&cmd) {
                continue;
            }
            let is_last = self.cursor >= target;
            self.process(&cmd, is_last, &mut out);
        }
        out
    }

    /// Rebuild the engine position for a mid-scene save: loads `scene` and
    /// re-runs every command up to `target` (the saved cursor), emitting only
    /// the final display command. `wait` commands already elapsed are skipped.
    pub fn resume(&mut self, scene: &str, target: usize) -> Vec<Cmd> {
        self.load_scene(scene);
        let mut out = Vec::new();
        while self.cursor < target && !self.waiting_for_choice {
            let cmd = self.script[self.cursor].clone();
            self.cursor += 1;
            if is_wait_cmd(&cmd) {
                continue;
            }
            let is_last = self.cursor >= target;
            self.process(&cmd, is_last, &mut out);
        }
        out
    }

    pub fn pick_choice(&mut self, index: usize) -> Vec<Cmd> {
        let mut out = Vec::new();
        if !self.waiting_for_choice {
            return out;
        }
        let choice_cmd = self.script[self.cursor.saturating_sub(1)].clone();
        let mut chosen: Option<Choice> = None;
        if let Some(opts) = choice_cmd.get("options").and_then(|o| o.as_array()) {
            if let Some(opt) = opts.get(index) {
                let label = opt.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string();
                let jump = opt
                    .get("jump")
                    .and_then(|j| j.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut set = HashMap::new();
                if let Some(m) = opt.get("set").and_then(|s| s.as_object()) {
                    for (k, v) in m {
                        set.insert(k.clone(), v.as_str().unwrap_or("true").to_string());
                    }
                }
                chosen = Some(Choice { label, jump, set });
            }
        }
        self.waiting_for_choice = false;
        let jump;
        if let Some(c) = chosen {
            for (k, v) in &c.set {
                state::player_mut().set_flag(k, v);
            }
            jump = c.jump;
        } else {
            jump = String::new();
        }
        if jump.is_empty() {
            self.advance_into(&mut out);
        } else {
            self.load_scene(&jump);
            self.advance_into(&mut out);
        }
        out
    }

    /// Process one command; push events onto `out`. Returns `true` when the
    /// caller should keep auto-advancing. `replay_line` controls whether
    /// `line` commands emit their event (false during a language reload,
    /// except for the final command).
    fn process(&mut self, cmd: &Value, replay_line: bool, out: &mut Vec<Cmd>) -> bool {
        let ty = cmd.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match ty {
            "line" => {
                let speaker = cmd
                    .get("speaker")
                    .and_then(|s| s.as_str())
                    .filter(|s| *s != "null")
                    .map(|s| s.to_string());
                let text = cmd
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                if replay_line {
                    out.push(Cmd::Line { speaker, text });
                }
                false
            }
            "bg" => {
                if let Some(f) = cmd.get("file").and_then(|f| f.as_str()) {
                    out.push(Cmd::Bg(f.to_string()));
                }
                true
            }
            "music" => {
                if let Some(f) = cmd.get("file").and_then(|f| f.as_str()) {
                    out.push(Cmd::Music(f.to_string()));
                }
                true
            }
            "sfx" => {
                if let Some(f) = cmd.get("file").and_then(|f| f.as_str()) {
                    out.push(Cmd::Sfx(f.to_string()));
                }
                true
            }
            "char" => {
                let name = cmd.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                let expr = cmd.get("expr").and_then(|e| e.as_str()).unwrap_or("normal").to_string();
                let pos = cmd.get("pos").and_then(|p| p.as_str()).unwrap_or("center").to_string();
                let visible = cmd.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                out.push(Cmd::Char { name, expr, pos, visible });
                true
            }
            "choice" => {
                let mut choices = Vec::new();
                if let Some(opts) = cmd.get("options").and_then(|o| o.as_array()) {
                    for opt in opts {
                        let label = opt.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string();
                        let jump = opt
                            .get("jump")
                            .and_then(|j| j.as_str())
                            .unwrap_or("")
                            .to_string();
                        let mut set = HashMap::new();
                        if let Some(m) = opt.get("set").and_then(|s| s.as_object()) {
                            for (k, v) in m {
                                set.insert(k.clone(), v.as_str().unwrap_or("true").to_string());
                            }
                        }
                        choices.push(Choice { label, jump, set });
                    }
                }
                self.waiting_for_choice = true;
                out.push(Cmd::Choices(choices));
                false
            }
            "jump" => {
                if let Some(scene) = cmd.get("scene").and_then(|s| s.as_str()) {
                    self.load_scene(scene);
                }
                true
            }
            "if" => {
                let flag = cmd.get("flag").and_then(|f| f.as_str()).unwrap_or("");
                let equals = cmd.get("equals").and_then(|e| e.as_str());
                let not_equals = cmd.get("not").and_then(|e| e.as_str());
                let jump = cmd.get("jump").and_then(|j| j.as_str()).unwrap_or("");
                let current = state::player().flag(flag).cloned();
                let matches = if let Some(e) = equals {
                    current.as_deref() == Some(e)
                } else if let Some(n) = not_equals {
                    current.as_deref() != Some(n)
                } else {
                    current.is_some()
                };
                if matches && !jump.is_empty() {
                    self.load_scene(jump);
                }
                true
            }
            "set" => {
                let flag = cmd.get("flag").and_then(|f| f.as_str()).unwrap_or("");
                let value = cmd.get("value").and_then(|v| v.as_str()).unwrap_or("true");
                state::player_mut().set_flag(flag, value);
                true
            }
            "wait" => {
                let secs = cmd.get("seconds").and_then(|s| s.as_f64()).unwrap_or(0.0) as f32;
                if secs > 0.0 {
                    out.push(Cmd::Wait(secs));
                }
                false
            }
            "poem" => {
                out.push(Cmd::Poem);
                false
            }
            "actcard" => {
                let title = cmd.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
                let subtitle = cmd.get("subtitle").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let next = cmd.get("next").and_then(|n| n.as_str()).unwrap_or("").to_string();
                out.push(Cmd::ActCard { title, subtitle, next });
                false
            }
            "end" => {
                out.push(Cmd::End);
                false
            }
            _ => {
                crate::log::warn(&format!("[script] unknown command: {ty}"));
                true
            }
        }
    }
}

fn is_wait_cmd(cmd: &Value) -> bool {
    cmd.get("type").and_then(|t| t.as_str()) == Some("wait")
}

fn read_script(local_name: &str) -> Vec<Value> {    for path in asset_candidates(&format!("scripts/{local_name}.json")) {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                if let Value::Array(arr) = v {
                    return arr;
                }
            }
        }
    }
    crate::log::warn(&format!("[script] not found: {local_name}"));
    Vec::new()
}

/// Returns candidate absolute paths for an asset, checked in order.
pub fn asset_candidates(rel: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(format!("{}/assets/{rel}", cwd.display()));
    }
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        out.push(format!("{dir}/assets/{rel}"));
    }
    // Packaged installs: fall back to the folder next to the executable (and
    // the macOS `.app` Resources folder) so the game works no matter where it
    // is launched from.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(format!("{}/assets/{rel}", dir.display()));
            out.push(format!("{}/../Resources/assets/{rel}", dir.display()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::asset_candidates;

    #[test]
    fn candidates_include_executable_dir() {
        let exe = std::env::current_exe().expect("current exe");
        let dir = exe.parent().expect("exe dir");
        let rel = format!("{}/assets/scripts/x.json", dir.display());
        let candidates = asset_candidates("scripts/x.json");
        assert!(candidates.iter().any(|c| c == &rel));
    }
}
