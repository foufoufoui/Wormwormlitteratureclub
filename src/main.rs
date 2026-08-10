//! Entry point: owns the screens and drives the game loop.

mod assets;
mod audio;
mod dialogue;
mod event;
mod game;
mod lang;
mod log;
mod menu;
mod overlay;
mod poem;
mod save;
mod settings;
mod state;
mod ui;

use macroquad::prelude::*;

use event::ScreenEvent;
use ui::{W, H};

enum Screen {
    Menu(menu::Menu),
    NameEntry(menu::NameEntry),
    Game(game::Game),
    Poem(poem::Poem),
    ActCard(menu::ActCard),
    Credits(menu::Credits),
}

/// App state kept outside the screens so it survives transitions.
struct App {
    screen: Screen,
    /// The game is parked here while the poem minigame runs.
    game: Option<Box<game::Game>>,
}

impl App {
    fn new() -> App {
        App {
            screen: Screen::Menu(menu::Menu::new()),
            game: None,
        }
    }

    async fn run(&mut self) {
        // Fixed 1280x720 virtual canvas: every screen draws into this render
        // target, which is then presented scaled to fit the real window. This
        // keeps the UI/mouse mapping constant regardless of window size or
        // fullscreen, with black letterbox bars when the aspect differs.
        let canvas = render_target(W as u32, H as u32);
        canvas.texture.set_filter(FilterMode::Linear);
        let mut canvas_cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, W, H));
        canvas_cam.render_target = Some(canvas.clone());

        loop {
            // Draw the current screen into the virtual canvas.
            set_camera(&canvas_cam);
            let current = std::mem::replace(&mut self.screen, Screen::Menu(menu::Menu::new()));
            self.screen = match current {
                Screen::Menu(mut m) => match m.frame() {
                    ScreenEvent::None
                    | ScreenEvent::Poem
                    | ScreenEvent::Credits
                    | ScreenEvent::ToMenu
                    | ScreenEvent::ActCard(_, _, _) => Screen::Menu(m),
                    ScreenEvent::StartScene(data) => Screen::Game(game::Game::from_save(&data)),
                    ScreenEvent::StartGame => {
                        state::reset();
                        Screen::NameEntry(menu::NameEntry::new())
                    }
                    ScreenEvent::Quit => break,
                },
                Screen::NameEntry(mut n) => {
                    if n.frame() {
                        Screen::Game(game::Game::new(&n.player_name(), "act1_scene1"))
                    } else {
                        Screen::NameEntry(n)
                    }
                }
                Screen::Game(mut g) => match g.frame() {
                    ScreenEvent::None | ScreenEvent::StartGame | ScreenEvent::Quit => Screen::Game(g),
                    ScreenEvent::Poem => {
                        self.game = Some(Box::new(g));
                        Screen::Poem(poem::Poem::new())
                    }
                    ScreenEvent::Credits => {
                        audio::stop_bgm();
                        Screen::Credits(menu::Credits::new())
                    }
                    ScreenEvent::ToMenu => {
                        audio::stop_bgm();
                        Screen::Menu(menu::Menu::new())
                    }
                    ScreenEvent::ActCard(title, subtitle, next) => {
                        Screen::ActCard(menu::ActCard::new(&title, &subtitle, &next))
                    }
                    ScreenEvent::StartScene(data) => Screen::Game(game::Game::from_save(&data)),
                },
                Screen::Poem(mut p) => {
                    if p.frame() {
                        let g = self.game.take().expect("game parked during poem");
                        Screen::Game(*g)
                    } else {
                        Screen::Poem(p)
                    }
                }
                Screen::ActCard(mut c) => {
                    if let Some(next) = c.frame() {
                        let name = state::player().player_name.clone();
                        Screen::Game(game::Game::new(&name, &next))
                    } else {
                        Screen::ActCard(c)
                    }
                }
                Screen::Credits(mut c) => {
                    if c.frame() {
                        audio::stop_bgm();
                        Screen::Menu(menu::Menu::new())
                    } else {
                        Screen::Credits(c)
                    }
                }
            };
            // Present the canvas letterboxed onto the window.
            set_default_camera();
            clear_background(BLACK);
            let scale = (screen_width() / W).min(screen_height() / H).max(0.0001);
            let dw = W * scale;
            let dh = H * scale;
            draw_texture_ex(
                &canvas.texture,
                (screen_width() - dw) * 0.5,
                (screen_height() - dh) * 0.5,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(dw, dh)),
                    flip_y: true,
                    ..Default::default()
                },
            );
            next_frame().await;
        }
    }
}

/// The game renders to a fixed 1280x720 canvas that is letterboxed onto the
/// window, so the window can safely be resized or put into fullscreen without
/// breaking the mouse-to-canvas mapping.
fn window_conf() -> macroquad::miniquad::conf::Conf {
    macroquad::miniquad::conf::Conf {
        window_title: "Worm Worm Literature Club".to_string(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Present a frame right away so the window appears and stays responsive
    // while assets (GIF frames, MP3s) are decoded on the main thread.
    splash(lang::t("splash.loading")).await;

    let a = assets::Assets::load().await;
    audio::init(a.sounds.clone());
    assets::init(a);

    let mut app = App::new();
    app.run().await;
}

/// Draw a full-screen loading splash and pump one frame.
async fn splash(text: &str) {
    clear_background(Color::from_rgba(16, 6, 22, 255));
    let font = get_default_font();
    draw_text_ex(
        text,
        60.0,
        90.0,
        TextParams {
            font: Some(&font),
            font_size: 30,
            color: Color::from_rgba(255, 220, 240, 255),
            ..Default::default()
        },
    );
    next_frame().await;
}
