//! Renders every screen into an in-memory terminal and drives the app with
//! key events (no real terminal needed).

use dispatch112::bot::{Bot, BotProfile};
use dispatch112::cli::Options;
use dispatch112::i18n::Lang;
use dispatch112::sim::Difficulty;
use dispatch112::ui::{render, App, Overlay, Page, Screen};
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use std::path::PathBuf;
use std::time::Duration;

fn temp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("dispatch112-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn key(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn screen_text(app: &App, w: u16, h: u16) -> String {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| render(f, app)).unwrap();
    let buf = term.backend().buffer().clone();
    let mut s = String::new();
    for y in 0..h {
        for x in 0..w {
            s.push_str(buf[(x, y)].symbol());
        }
        s.push('\n');
    }
    s
}

fn new_app(lang: Lang, name: &str) -> App {
    let opts = Options { lang: Some(lang), difficulty: Some(Difficulty::Rookie), seed: Some(5), ..Default::default() };
    App::new(&opts, temp_dir(name))
}

#[test]
fn too_small_terminal_shows_a_message() {
    let app = new_app(Lang::En, "small");
    let s = screen_text(&app, 80, 24);
    assert!(s.contains("Please enlarge the terminal"), "{s}");
}

#[test]
fn full_flow_title_to_report_in_every_language() {
    for lang in Lang::ALL {
        let mut app = new_app(lang, &format!("flow-{}", lang.code()));
        let title = screen_text(&app, 100, 30);
        assert!(title.contains("██"), "logo missing");
        // Title -> New game -> briefing.
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Briefing);
        screen_text(&app, 100, 30);
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Shift);
        // Monday shows Zuhra-opa's welcome tip after the first tick.
        app.step();
        assert!(matches!(app.overlay, Some(Overlay::Tip(_))));
        screen_text(&app, 100, 30);
        // Dismiss tips as they come and play a few minutes by hand.
        for _ in 0..60 {
            while app.overlay.is_some() {
                key(&mut app, KeyCode::Enter);
            }
            app.step();
            key(&mut app, KeyCode::Char('1'));
            key(&mut app, KeyCode::Enter);
        }
        while app.overlay.is_some() {
            key(&mut app, KeyCode::Enter);
        }
        let s = screen_text(&app, 100, 30);
        assert!(s.contains("Yangiobod") || s.contains("Янгиобод"));
        // Bigger terminal, help overlay and pause.
        key(&mut app, KeyCode::Char('?'));
        assert_eq!(app.overlay, Some(Overlay::Help));
        screen_text(&app, 140, 45);
        key(&mut app, KeyCode::Esc);
        key(&mut app, KeyCode::Char(' '));
        assert!(app.paused);
        app.update(Duration::from_secs(5));
        key(&mut app, KeyCode::Char(' '));
        // Finish the shift with a competent dispatcher at the console.
        let mut bot = Bot::new(BotProfile::good(), 1);
        while app.screen == Screen::Shift {
            while app.overlay.is_some() {
                key(&mut app, KeyCode::Enter);
            }
            if let Some(s) = app.shift.as_mut() {
                for cmd in bot.decide(s) {
                    let _ = s.apply(&cmd);
                }
            }
            app.step();
        }
        assert_eq!(app.screen, Screen::Report(Page::Grade));
        for page in [Page::Grade, Page::Zuhra, Page::Inspector] {
            assert_eq!(app.screen, Screen::Report(page));
            screen_text(&app, 100, 30);
            key(&mut app, KeyCode::Enter);
        }
        if app.screen == Screen::Report(Page::Upgrade) {
            screen_text(&app, 100, 30);
            key(&mut app, KeyCode::Char('1'));
            assert_eq!(app.screen, Screen::Briefing);
            assert!(app.saved.is_some(), "progress saved between shifts");
        } else {
            assert_eq!(app.screen, Screen::Ending);
            screen_text(&app, 100, 30);
        }
    }
}

#[test]
fn language_switch_and_quit_confirm() {
    let mut app = new_app(Lang::Uz, "lang");
    key(&mut app, KeyCode::Char('l'));
    assert_eq!(app.lang, Lang::Ru);
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.screen, Screen::Shift);
    while app.overlay.is_some() || app.shift.as_ref().unwrap().tick == 0 {
        if app.overlay.is_some() {
            key(&mut app, KeyCode::Esc);
        } else {
            app.step();
        }
    }
    key(&mut app, KeyCode::Char('q'));
    assert_eq!(app.overlay, Some(Overlay::QuitConfirm));
    key(&mut app, KeyCode::Char('n'));
    assert!(!app.quit);
    key(&mut app, KeyCode::Char('q'));
    key(&mut app, KeyCode::Char('y'));
    assert!(app.quit);
}

#[test]
fn autoplay_runs_a_shift_on_its_own() {
    let opts = Options { lang: Some(Lang::En), seed: Some(9), autoplay: true, ..Default::default() };
    let mut app = App::new(&opts, temp_dir("auto"));
    // Simulated wall-clock time: title -> briefing -> shift.
    for _ in 0..4 {
        app.update(Duration::from_secs(3));
    }
    assert_eq!(app.screen, Screen::Shift);
    for _ in 0..2000 {
        app.update(Duration::from_millis(500));
        if app.screen != Screen::Shift {
            break;
        }
    }
    assert!(matches!(app.screen, Screen::Report(_) | Screen::Ending), "{:?}", app.screen);
    let s = app.record.as_ref().unwrap();
    assert!(s.stats.resolved > 10, "the bot should handle calls");
    assert!(app.saved.is_none(), "autoplay must not touch the save file");
}
