// ANCHOR: all
use std::{error::Error, io};

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

mod app;
mod ui;
mod network;

use rust_cli_roguelike::common::protocol;
use crate::{
    app::{App, CurrentScreen, GameMode, NetworkClient},
    ui::ui,
};

// ANCHOR: main_all
// ANCHOR: setup_boilerplate
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
    let res = run_app(&mut terminal, app).await;
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
    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
// ANCHOR_END: final_print
// ANCHOR_END: main_all

// ANCHOR: run_app_all
// ANCHOR: run_method_signature
async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        // Process network messages if in multiplayer mode
        if app.game_mode == GameMode::MultiPlayer {
            app.process_network_messages();
        }

        terminal.draw(|f| ui(f, &mut app))?;

        // Use a timeout for event reading so we can process network messages more frequently
        let timeout = std::time::Duration::from_millis(50); // 20 FPS
        if let Ok(has_event) = event::poll(timeout) {
            if has_event {
                if let Event::Key(key) = event::read()? {
                    if key.kind == ratatui::crossterm::event::KeyEventKind::Press {
                        match app.current_screen {
                            CurrentScreen::MainMenu => {
                                if app.main_menu_state.username_input_mode {
                                    // Handle username input
                                    match key.code {
                                        KeyCode::Enter => {
                                            app.finish_username_input();
                                        }
                                        KeyCode::Esc => {
                                            app.cancel_username_input();
                                        }
                                        KeyCode::Backspace => {
                                            app.remove_char_from_username();
                                        }
                                        KeyCode::Char(c) => {
                                            app.add_char_to_username(c);
                                        }
                                        _ => {}
                                    }
                                } else {
                                    // Handle menu navigation
                                    match key.code {
                                        KeyCode::Up => {
                                            if app.main_menu_state.selected_option > 0 {
                                                app.main_menu_state.selected_option -= 1;
                                            }
                                        }
                                        KeyCode::Down => {
                                            if app.main_menu_state.selected_option < 3 { // Updated for 4 options
                                                app.main_menu_state.selected_option += 1;
                                            }
                                        }
                                        KeyCode::Enter => {
                                            match app.main_menu_state.selected_option {
                                                0 => {
                                                    // Single Player
                                                    app.start_single_player();
                                                }
                                                1 => {
                                                    // Multiplayer - try to connect
                                                    app.main_menu_state.connecting = true;
                                                    match NetworkClient::connect(&app.server_address, app.player_name.clone()).await {
                                                        Ok(client) => {
                                                            app.start_multiplayer(client);
                                                        }
                                                        Err(e) => {
                                                            app.main_menu_state.connecting = false;
                                                            app.main_menu_state.connection_error = Some(format!("Failed to connect: {}", e));
                                                        }
                                                    }
                                                }
                                                2 => {
                                                    // Set Username
                                                    app.start_username_input();
                                                }
                                                3 => {
                                                    // Quit
                                                    app.should_quit = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        KeyCode::Char('q') => {
                                            app.should_quit = true;
                                        }
                                        _ => {}
                                    }
                                }
                            },
                            CurrentScreen::Game => {
                                if app.chat_input_mode {
                                    // Handle chat input mode
                                    match key.code {
                                        KeyCode::Enter => {
                                            app.send_chat_message();
                                        }
                                        KeyCode::Esc => {
                                            app.close_chat();
                                        }
                                        KeyCode::Backspace => {
                                            app.remove_char_from_chat();
                                        }
                                        KeyCode::Char(c) => {
                                            app.add_char_to_chat(c);
                                        }
                                        _ => {}
                                    }
                                } else {
                                    // Handle normal game controls
                                    match key.code {
                                        KeyCode::Char('q') => {
                                            if app.game_mode == GameMode::MultiPlayer {
                                                app.disconnect();
                                            } else {
                                                app.current_screen = CurrentScreen::Exiting;
                                            }
                                        }
                                        KeyCode::Char('i') => {
                                            app.open_inventory();
                                        }
                                        KeyCode::Char('c') => {
                                            app.open_chat();
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
                                    }
                                }
                            },
                            CurrentScreen::Inventory => match key.code {
                                KeyCode::Char('g') | KeyCode::Esc => {
                                    app.close_inventory();
                                }
                                KeyCode::Char('q') => {
                                    if app.game_mode == GameMode::MultiPlayer {
                                        app.disconnect();
                                    } else {
                                        app.current_screen = CurrentScreen::Exiting;
                                    }
                                }
                                _ => {}
                            },
                            CurrentScreen::Chat => match key.code {
                                KeyCode::Enter => {
                                    app.send_chat_message();
                                }
                                KeyCode::Esc => {
                                    app.close_chat();
                                }
                                KeyCode::Backspace => {
                                    app.remove_char_from_chat();
                                }
                                KeyCode::Char(c) => {
                                    app.add_char_to_chat(c);
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
            }
        }

        if app.should_quit {
            if app.game_mode == GameMode::MultiPlayer {
                app.disconnect();
            }
            break;
        }
    }
    Ok(())
}
// ANCHOR_END: run_app_all

// ANCHOR_END: all