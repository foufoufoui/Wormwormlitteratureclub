//! Screen-transition events emitted by each screen every frame.

use crate::save::SaveData;

#[derive(Debug)]
pub enum ScreenEvent {
    None,
    ToMenu,
    /// Start a new game (name is entered on the name-entry screen).
    StartGame,
    /// Resume a saved game; carries the full save snapshot so the dialogue
    /// position, background, characters and current line are restored.
    StartScene(Box<SaveData>),
    /// Enter the poem minigame.
    Poem,
    /// Go to the credits.
    Credits,
    /// Show an act card, then continue to the given scene.
    ActCard(String, String, String),
    /// Quit the application.
    Quit,
}

impl Default for ScreenEvent {
    fn default() -> Self {
        ScreenEvent::None
    }
}
