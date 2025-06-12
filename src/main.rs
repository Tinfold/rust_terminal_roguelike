// ANCHOR: all
use std::{error::Error, io};

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

mod app;
mod ui;
mod terrain;
use crate::{
    app::{App, CurrentScreen},
    ui::ui,
};

// ANCHOR: main_all
// ANCHOR: setup_boilerplate
fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stderr = io::stderr(); // This is a special case. Normally using stdout is fine
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    // ANCHOR_END: setup_boilerplate
    // ANCHOR: application_startup
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);
    // ANCHOR_END: application_startup

    // ANCHOR: ending_boilerplate
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    // ANCHOR_END: ending_boilerplate

    // ANCHOR: final_print
    if let Ok(do_print) = res {

    } else if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
// ANCHOR_END: final_print
// ANCHOR_END: main_all

// ANCHOR: run_app_all
// ANCHOR: run_method_signature
fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match app.current_screen {
                    CurrentScreen::Game => match key.code {
                        KeyCode::Char('q') => {
                            app.current_screen = CurrentScreen::Exiting;
                        }
                        KeyCode::Char('i') => {
                            app.current_screen = CurrentScreen::Inventory;
                        }
                        KeyCode::Char('e') => {
                            app.enter_dungeon();
                        }
                        KeyCode::Char('x') => {
                            app.exit_dungeon();
                        }
                        // Movement keys (vi-style)
                        KeyCode::Char('h') | KeyCode::Left => {
                            app.move_player(-1, 0);
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.move_player(0, 1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.move_player(0, -1);
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            app.move_player(1, 0);
                        }
                        // Diagonal movement
                        KeyCode::Char('y') => app.move_player(-1, -1),
                        KeyCode::Char('u') => app.move_player(1, -1),
                        KeyCode::Char('b') => app.move_player(-1, 1),
                        KeyCode::Char('n') => app.move_player(1, 1),
                        _ => {}
                    },
                    CurrentScreen::Inventory => match key.code {
                        KeyCode::Char('g') | KeyCode::Esc => {
                            app.current_screen = CurrentScreen::Game;
                        }
                        KeyCode::Char('q') => {
                            app.current_screen = CurrentScreen::Exiting;
                        }
                        _ => {}
                    },
                    CurrentScreen::Exiting => match key.code {
                        KeyCode::Char('y') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('n') | KeyCode::Esc => {
                            app.current_screen = CurrentScreen::Game;
                        }
                        _ => {}
                    },
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
// ...existing code...
// ANCHOR: run_app_all

// ANCHOR_END: all