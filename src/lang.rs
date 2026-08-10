//! UI strings and script-name localisation (EN / FR).

use std::collections::HashMap;

pub fn is_french() -> bool {
    crate::settings::settings().lang == "fr"
}

/// Returns the script filename to load, inserting `_fr` suffix if needed.
pub fn script_name(base: &str) -> String {
    if is_french() {
        format!("{base}_fr")
    } else {
        base.to_string()
    }
}

pub fn t(key: &'static str) -> &'static str {
    let table: &HashMap<&'static str, &'static str> = if is_french() { &FR } else { &EN };
    table.get(key).copied().unwrap_or(key)
}

macro_rules! table {
    ( $($k:literal => $v:literal),* $(,)? ) => {{
        let mut m = HashMap::new();
        $( m.insert($k, $v); )*
        m
    }};
}

static EN: std::sync::LazyLock<HashMap<&'static str, &'static str>> = std::sync::LazyLock::new(|| table! {
    "btn.menu" => "Menu",
    "btn.auto" => "Auto",
    "btn.save" => "Save",
    "btn.settings" => "Settings",
    "btn.close" => "Close",
    "btn.back" => "Back",
    "btn.newgame" => "New Game",
    "btn.loadgame" => "Load Game",
    "btn.quit" => "Quit",
    "btn.fullscreen" => "Fullscreen",
    "btn.windowed" => "Windowed",
    "settings.title" => "Settings",
    "settings.speed" => "Text Speed",
    "settings.music" => "Music Volume",
    "settings.sfx" => "SFX Volume",
    "settings.lang" => "Language",
    "settings.fast" => "Fast",
    "settings.slow" => "Slow",
    "settings.off" => "Off",
    "settings.max" => "Max",
    "save.title" => "Save Game",
    "load.title" => "Load Game",
    "save.empty" => "Empty",
    "save.slot" => "Slot",
    "save.overwrite" => "Overwrite this save?",
    "save.overwrite_btn" => "Overwrite",
    "save.cancel" => "Cancel",
    "save.save_btn" => "Save",
    "save.load_btn" => "Load",
    "save.delete" => "X",
    "menu.confirm" => "Return to main menu?\nUnsaved progress will be lost.",
    "name.prompt" => "What is your name?",
    "name.sub" => "(This is how you'll appear in the story)",
    "name.placeholder" => "Enter your name...",
    "name.confirm" => "Let's go!",
    "name.title" => "Welcome",
    "name.start" => "Begin",
    "name.default" => "Player",
    "title.game" => "Worm Worm Literature Club",
    "title.sub" => "~ Praise be to Scatha ~",
    "confirm.leave" => "Leave",
    "confirm.stay" => "Stay",
    "credits.skip" => "Press Enter to skip",
    "poem.title" => "~ Write Your Poem ~",
    "poem.sub" => "Words you choose here shape how the club sees your heart.",
    "poem.prompt" => "Choose a word to complete the line:",
    "poem.continue" => "Continue",
    "poem.finished" => "Your poem is complete.",
    "splash.loading" => "Loading\u{2026}",
});

static FR: std::sync::LazyLock<HashMap<&'static str, &'static str>> = std::sync::LazyLock::new(|| table! {
    "btn.menu" => "Menu",
    "btn.auto" => "Auto",
    "btn.save" => "Sauvegarder",
    "btn.settings" => "Param\u{00E8}tres",
    "btn.close" => "Fermer",
    "btn.back" => "Retour",
    "btn.newgame" => "Nouvelle Partie",
    "btn.loadgame" => "Charger",
    "btn.quit" => "Quitter",
    "btn.fullscreen" => "Plein \u{00E9}cran",
    "btn.windowed" => "Fen\u{00EA}tr\u{00E9}",
    "settings.title" => "Param\u{00E8}tres",
    "settings.speed" => "Vitesse du texte",
    "settings.music" => "Volume musique",
    "settings.sfx" => "Volume effets",
    "settings.lang" => "Langue",
    "settings.fast" => "Rapide",
    "settings.slow" => "Lent",
    "settings.off" => "Muet",
    "settings.max" => "Max",
    "save.title" => "Sauvegarder",
    "load.title" => "Charger",
    "save.empty" => "Vide",
    "save.slot" => "Slot",
    "save.overwrite" => "\u{00C9}craser cette sauvegarde ?",
    "save.overwrite_btn" => "\u{00C9}craser",
    "save.cancel" => "Annuler",
    "save.save_btn" => "Sauver",
    "save.load_btn" => "Charger",
    "save.delete" => "X",
    "menu.confirm" => "Retourner au menu principal ?\nLa progression non sauvegard\u{00E9}e sera perdue.",
    "name.prompt" => "Quel est ton pr\u{00E9}nom ?",
    "name.sub" => "(C'est ainsi que tu appara\u{00EE}tras dans l'histoire)",
    "name.placeholder" => "Entre ton pr\u{00E9}nom...",
    "name.confirm" => "C'est parti !",
    "name.title" => "Bienvenue",
    "name.start" => "Commencer",
    "name.default" => "Joueur",
    "title.game" => "Worm Worm Literature Club",
    "title.sub" => "~ Louange \u{00E0} Scatha ~",
    "confirm.leave" => "Partir",
    "confirm.stay" => "Rester",
    "credits.skip" => "Appuyez sur Entr\u{00E9}e pour passer",
    "poem.title" => "~ \u{00C9}cris ton po\u{00E8}me ~",
    "poem.sub" => "Les mots choisis montrent au club ce que ton coeur ressent.",
    "poem.prompt" => "Choisis un mot pour compl\u{00E9}ter le vers :",
    "poem.continue" => "Continuer",
    "poem.finished" => "Ton po\u{00E8}me est termin\u{00E9}.",
    "splash.loading" => "Chargement\u{2026}",
});
