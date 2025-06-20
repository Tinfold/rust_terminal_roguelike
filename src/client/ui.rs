use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen, MapType, Tile, GameMode};
use rust_cli_roguelike::common::game_logic::GameLogic;

pub fn ui(frame: &mut Frame, app: &mut App) {
    match app.current_screen {
        CurrentScreen::MainMenu => render_main_menu(frame, app),
        CurrentScreen::Chat => render_chat_screen(frame, app),
        _ => render_game_ui(frame, app),
    }
}

fn render_main_menu(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Menu
            Constraint::Length(3),  // Status/Error
        ])
        .split(frame.area());

    // Title
    let title = Paragraph::new(Text::styled(
        "🗡️  MULTIPLAYER ROGUELIKE  🛡️",
        Style::default().fg(Color::Yellow),
    ))
    .block(Block::default().borders(Borders::ALL))
    .wrap(Wrap { trim: false });

    frame.render_widget(title, chunks[0]);

    // Menu options
    let menu_items = if app.main_menu_state.username_input_mode {
        vec!["[Press Enter to confirm, Esc to cancel]"]
    } else {
        vec![
            "Single Player",
            "Multiplayer", 
            "Set Username",
            "Quit",
        ]
    };

    let mut menu_list_items = Vec::<ListItem>::new();
    
    if app.main_menu_state.username_input_mode {
        // Username input mode
        menu_list_items.push(ListItem::new(Line::from(Span::styled(
            format!("Username: {}", app.main_menu_state.username_input),
            Style::default().fg(Color::Yellow),
        ))));
        menu_list_items.push(ListItem::new(Line::from(Span::styled(
            "[Press Enter to confirm, Esc to cancel]",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        // Normal menu
        for (i, item) in menu_items.iter().enumerate() {
            let style = if i == app.main_menu_state.selected_option {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            
            let prefix = if i == app.main_menu_state.selected_option { "▶ " } else { "  " };
            menu_list_items.push(ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, item),
                style,
            ))));
        }
    }

    let menu_list = List::new(menu_list_items)
        .block(Block::default().borders(Borders::ALL).title(
            if app.main_menu_state.username_input_mode {
                "Enter Username"
            } else {
                "Select Option (↑/↓ to select, Enter to confirm)"
            }
        ));

    frame.render_widget(menu_list, chunks[1]);

    // Status/Error
    let status_text = if app.main_menu_state.connecting {
        format!("Connecting to server {}...", app.server_address)
    } else if let Some(ref error) = app.main_menu_state.connection_error {
        format!("Error: {}", error)
    } else {
        format!("Server: {} | Player: {} | Press Q to quit", app.server_address, app.player_name)
    };

    let status_color = if app.main_menu_state.connection_error.is_some() {
        Color::Red
    } else if app.main_menu_state.connecting {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let status = Paragraph::new(Text::styled(
        status_text,
        Style::default().fg(status_color),
    ))
    .block(Block::default().borders(Borders::ALL).title("Status"));

    frame.render_widget(status, chunks[2]);
}

fn render_game_ui(frame: &mut Frame, app: &mut App) {
    // Create the layout sections based on chat input mode
    let constraints = if app.chat_input_mode && app.game_mode == GameMode::MultiPlayer {
        vec![
            Constraint::Length(3),  // Status bar
            Constraint::Min(20),    // Game area (minimum height)
            Constraint::Length(3),  // Chat input bar (full width)
            Constraint::Length(5),  // Message log
        ]
    } else {
        vec![
            Constraint::Length(3),  // Status bar
            Constraint::Min(20),    // Game area (minimum height)
            Constraint::Length(5),  // Message log
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    // Status bar showing player stats and current screen
    let mode_text = match app.game_mode {
        GameMode::SinglePlayer => "Single Player",
        GameMode::MultiPlayer => "Multiplayer",
    };
    
    let status_text = if app.game_mode == GameMode::MultiPlayer {
        format!(
            "HP: {}/{} | Turn: {} | Map: {} | Position: ({}, {}) | Mode: {} | Controls: HJKL/Arrows (move), E (enter dungeon), X (exit dungeon), I (inventory), C (chat), Q (quit)",
            app.player.hp, 
            app.player.max_hp, 
            app.turn_count, 
            match app.current_map_type {
                MapType::Overworld => "Overworld",
                MapType::Dungeon => "Dungeon",
            },
            app.player.x,
            app.player.y,
            mode_text
        )
    } else {
        format!(
            "HP: {}/{} | Turn: {} | Map: {} | Position: ({}, {}) | Mode: {} | Controls: HJKL/Arrows (move), E (enter dungeon), X (exit dungeon), I (inventory), Q (quit)",
            app.player.hp, 
            app.player.max_hp, 
            app.turn_count, 
            match app.current_map_type {
                MapType::Overworld => "Overworld",
                MapType::Dungeon => "Dungeon",
            },
            app.player.x,
            app.player.y,
            mode_text
        )
    };
    
    let status_block = Block::default()
        .borders(Borders::ALL)
        .title("Status")
        .style(Style::default());

    let status = Paragraph::new(Text::styled(
        status_text,
        Style::default().fg(Color::White),
    ))
    .block(status_block);

    frame.render_widget(status, chunks[0]);

    // Game area - render based on current screen and mode
    match app.current_screen {
        CurrentScreen::MainMenu => unreachable!(), // Handled above
        CurrentScreen::Chat => unreachable!(), // Handled separately
        CurrentScreen::Game => {
            if app.game_mode == GameMode::MultiPlayer && !app.chat_messages.is_empty() {
                // Split game area horizontally to show chat widget
                let game_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(60),     // Game map (minimum width)
                        Constraint::Length(30),  // Chat widget (fixed width)
                    ])
                    .split(chunks[1]);
                
                render_game_map(frame, app, game_chunks[0]);
                render_chat_widget(frame, app, game_chunks[1]);
            } else {
                render_game_map(frame, app, chunks[1]);
            }
        },
        CurrentScreen::Inventory => render_inventory(frame, app, chunks[1]),
        CurrentScreen::Exiting => render_exit_screen(frame, app, chunks[1]),
    }

    // Chat input bar (if in chat input mode) - full width under game area
    if app.chat_input_mode && app.game_mode == GameMode::MultiPlayer {
        render_chat_input_bar(frame, app, chunks[2]);
        
        // Message log is now at index 3
        let mut message_items = Vec::<ListItem>::new();
        for message in app.messages.iter().rev().take(3) {
            message_items.push(ListItem::new(Line::from(Span::styled(
                message.clone(),
                Style::default().fg(Color::Cyan),
            ))));
        }

        let message_list = List::new(message_items)
            .block(Block::default().borders(Borders::ALL).title("Messages"));

        frame.render_widget(message_list, chunks[3]);
    } else {
        // Message log at normal position when not in chat input mode
        let mut message_items = Vec::<ListItem>::new();
        for message in app.messages.iter().rev().take(3) {
            message_items.push(ListItem::new(Line::from(Span::styled(
                message.clone(),
                Style::default().fg(Color::Cyan),
            ))));
        }

        let message_list = List::new(message_items)
            .block(Block::default().borders(Borders::ALL).title("Messages"));

        frame.render_widget(message_list, chunks[2]);
    }
}
fn render_game_map(frame: &mut Frame, app: &mut App, area: Rect) {
    // Calculate the viewport size (accounting for borders)
    let viewport_width = (area.width.saturating_sub(2)) as i32; // Subtract 2 for borders
    let viewport_height = (area.height.saturating_sub(2)) as i32; // Subtract 2 for borders
    
    // Ensure minimum viewport size and make width wider to utilize terminal space better
    let viewport_width = viewport_width.max(60); // Increased minimum width
    let viewport_height = viewport_height.max(20); // Increased minimum height
    
    // Calculate camera position to center on player
    let camera_x = app.player.x - viewport_width / 2;
    let camera_y = app.player.y - viewport_height / 2;
    
    // Update chunk manager with player position if available
    if let Some(ref mut chunk_manager) = app.chunk_manager {
        chunk_manager.update_player_position(app.player.x, app.player.y);
    }
    
    let mut lines = Vec::<Line>::new();
    
    for viewport_y in 0..viewport_height {
        let mut spans = Vec::<Span>::new();
        
        for viewport_x in 0..viewport_width {
            let world_x = camera_x + viewport_x;
            let world_y = camera_y + viewport_y;
            
            if world_x == app.player.x && world_y == app.player.y {
                // Player character with bright yellow foreground and dark background
                spans.push(Span::styled(
                    app.player.symbol.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::DarkGray)
                ));
            } else if let Some(other_player) = app.other_players.values().find(|p| p.x == world_x && p.y == world_y && p.current_map_type == app.current_map_type) {
                // Other players in multiplayer mode - only show players in the same map
                let player_color = Color::Rgb(other_player.color.0, other_player.color.1, other_player.color.2);
                spans.push(Span::styled(
                    other_player.symbol.to_string(),
                    Style::default()
                        .fg(player_color)
                ));
            } else {
                // Try to get tile from different sources based on game mode
                let tile = if app.game_mode == GameMode::SinglePlayer {
                    // Single player: use chunk manager for infinite terrain
                    if let Some(ref mut chunk_manager) = app.chunk_manager {
                        chunk_manager.get_tile(world_x, world_y)
                    } else {
                        // Fall back to traditional game map
                        app.game_map.tiles.get(&(world_x, world_y)).copied()
                    }
                } else {
                    // Multiplayer: check if in dungeon first, then use appropriate map source
                    if app.current_map_type == MapType::Dungeon {
                        // In dungeon: use the traditional game map
                        app.game_map.tiles.get(&(world_x, world_y)).copied()
                    } else {
                        // In overworld: try multiplayer chunks first, then traditional map
                        app.get_multiplayer_tile(world_x, world_y).or_else(|| 
                            app.game_map.tiles.get(&(world_x, world_y)).copied()
                        )
                    }
                };
                
                if let Some(tile) = tile {
                    // Check tile visibility using the new lighting system (for dungeons)
                    if app.current_map_type == MapType::Dungeon {
                        const LIGHT_RADIUS: i32 = 6; // Player's light radius
                        let visibility_state = app.game_map.get_tile_visibility_state_with_doors(
                            app.player.x, app.player.y, world_x, world_y, LIGHT_RADIUS, &app.player.opened_doors
                        );
                        
                        // Also check game logic visibility for exploration-based visibility (doors, etc.)
                        let game_logic_visible = GameLogic::is_tile_visible(&app.game_map, &app.player, world_x, world_y);
                        
                        if visibility_state.is_visible() || game_logic_visible {
                            let brightness = if visibility_state.is_visible() {
                                visibility_state.get_brightness()
                            } else {
                                0.3 // Dim lighting for exploration-visible tiles
                            };
                            let (base_style, character) = get_tile_style_and_char(tile);
                            
                            // Apply brightness to the tile color
                            let modified_style = apply_brightness_to_style(base_style, brightness);
                            spans.push(Span::styled(character.to_string(), modified_style));
                        } else {
                            // Hidden tile - show as dark space
                            spans.push(Span::styled(" ".to_string(), Style::default().bg(Color::Black)));
                        }
                    } else {
                        // In overworld, all tiles are always visible at full brightness
                        let (style, character) = get_tile_style_and_char(tile);
                        spans.push(Span::styled(character.to_string(), style));
                    }
                } else {
                    // Out of bounds or empty space - show void
                    spans.push(Span::styled(" ".to_string(), Style::default().bg(Color::Black)));
                }
            }
        }
        lines.push(Line::from(spans));
    }

    let title = match app.current_map_type {
        MapType::Overworld => {
            if app.game_mode == GameMode::MultiPlayer {
                let players_in_overworld = app.other_players.values().filter(|p| p.current_map_type == MapType::Overworld).count() + 1;
                format!("🌍 Overworld (Players: {})", players_in_overworld)
            } else {
                "🌍 Overworld".to_string()
            }
        },
        MapType::Dungeon => {
            if app.game_mode == GameMode::MultiPlayer {
                let players_in_dungeon = app.other_players.values().filter(|p| p.current_map_type == MapType::Dungeon).count() + 1;
                format!("🏰 Dungeon (Players: {})", players_in_dungeon)
            } else {
                "🏰 Dungeon".to_string()
            }
        },
    };

    let game_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default());

    let game_area = Paragraph::new(Text::from(lines))
        .block(game_block);

    frame.render_widget(game_area, area);
}

fn render_chat_screen(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Title
            Constraint::Min(10),     // Chat messages
            Constraint::Length(3),   // Input box
            Constraint::Length(2),   // Instructions
        ])
        .split(frame.area());

    // Title
    let title = Paragraph::new(Text::styled(
        "💬 Chat Window",
        Style::default().fg(Color::Yellow),
    ))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Chat messages with text wrapping
    let available_width = chunks[1].width.saturating_sub(4) as usize; // Account for borders and padding
    
    // Collect all messages first with their wrapping
    let mut chat_lines = Vec::new();
    
    for (player_name, message) in app.chat_messages.iter().rev().take(15) { // Show last 15 messages
        let full_message = format!("{}: {}", player_name, message);
        let wrapped_lines = wrap_text(&full_message, available_width);
        
        for (i, line) in wrapped_lines.iter().enumerate() {
            if i == 0 {
                // First line: show player name in their assigned color, message in white
                let name_end = player_name.len() + 2; // +2 for ": "
                
                // Find the player's color - check if it's current player (yellow) or other players
                let player_color = if *player_name == app.player_name {
                    Color::Yellow // Current player uses yellow like on the map
                } else {
                    app.other_players.values()
                        .find(|p| p.name == *player_name)
                        .map(|p| Color::Rgb(p.color.0, p.color.1, p.color.2))
                        .unwrap_or(Color::Cyan)
                };
                
                if line.len() > name_end {
                    chat_lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}: ", player_name),
                            Style::default().fg(player_color),
                        ),
                        Span::styled(
                            line[name_end..].to_string(),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                } else {
                    chat_lines.push(Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(player_color),
                    )));
                }
            } else {
                // Continuation lines: indent and show in white
                chat_lines.push(Line::from(Span::styled(
                    format!("  {}", line), // 2-space indent for wrapped lines
                    Style::default().fg(Color::White),
                )));
            }
        }
    }
    
    // Reverse to show in chronological order (oldest at top, newest at bottom)
    chat_lines.reverse();

    let chat_paragraph = Paragraph::new(Text::from(chat_lines))
        .block(Block::default().borders(Borders::ALL).title("Chat Messages"))
        .wrap(Wrap { trim: false });
    frame.render_widget(chat_paragraph, chunks[1]);

    // Input box with text wrapping
    let input_text = format!("> {}", app.chat_input);
    let input = Paragraph::new(Text::styled(
        input_text,
        Style::default().fg(Color::Green),
    ))
    .block(Block::default().borders(Borders::ALL).title("Type your message"))
    .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[2]);

    // Instructions
    let instructions = Paragraph::new(Text::styled(
        "Press Enter to send, Esc to close chat",
        Style::default().fg(Color::Gray),
    ))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[3]);
}

fn render_chat_widget(frame: &mut Frame, app: &App, area: Rect) {
    // Chat widget for multiplayer mode - use Paragraph with wrapping instead of List
    let mut chat_lines = Vec::<Line>::new();
    
    // Available width for text (accounting for borders and padding)
    let available_width = area.width.saturating_sub(4) as usize; // 2 for borders, 2 for padding
    let available_height = (area.height.saturating_sub(2)) as usize; // Account for borders
    
    // Process messages from newest to oldest, but collect them to reverse the order later
    let mut all_messages = Vec::new();
    let mut total_lines = 0;
    
    for (player_name, message) in app.chat_messages.iter().rev().take(15) {
        let full_message = format!("{}: {}", player_name, message);
        let wrapped_lines = wrap_text(&full_message, available_width);
        
        // Check if adding this message would exceed available height
        let lines_count = wrapped_lines.len();
        if total_lines + lines_count > available_height {
            break;
        }
        
        all_messages.push((player_name.clone(), wrapped_lines));
        total_lines += lines_count;
    }
    
    // Now process in chronological order (oldest first)
    for (player_name, wrapped_lines) in all_messages.iter().rev() {
        for (i, line) in wrapped_lines.iter().enumerate() {
            if i == 0 {
                // First line: show player name in their assigned color, message in white
                let name_end = player_name.len() + 2; // +2 for ": "
                
                // Find the player's color - check if it's current player (yellow) or other players
                let player_color = if *player_name == app.player_name {
                    Color::Yellow // Current player uses yellow like on the map
                } else {
                    app.other_players.values()
                        .find(|p| p.name == *player_name)
                        .map(|p| Color::Rgb(p.color.0, p.color.1, p.color.2))
                        .unwrap_or(Color::Cyan)
                };
                
                if line.len() > name_end {
                    chat_lines.push(Line::from(vec![
                        Span::styled(
                            format!("{}: ", player_name),
                            Style::default().fg(player_color),
                        ),
                        Span::styled(
                            line[name_end..].to_string(),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                } else {
                    chat_lines.push(Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(player_color),
                    )));
                }
            } else {
                // Continuation lines: indent and show in white
                chat_lines.push(Line::from(Span::styled(
                    format!("  {}", line), // 2-space indent for wrapped lines
                    Style::default().fg(Color::White),
                )));
            }
        }
    }

    let chat_title = format!("💬 Chat ({})", app.chat_messages.len());
    let chat_paragraph = Paragraph::new(Text::from(chat_lines))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(chat_title)
            .title_style(Style::default().fg(Color::Yellow)))
        .wrap(Wrap { trim: false });
    
    frame.render_widget(chat_paragraph, area);
}

fn get_tile_style_and_char(tile: Tile) -> (Style, char) {
    match tile {
        Tile::Floor => (
            Style::default().fg(Color::Gray),
            '.'
        ),
        Tile::Wall => (
            Style::default().fg(Color::White).bg(Color::DarkGray),
            '#'
        ),
        Tile::Empty => (
            Style::default(),
            ' '
        ),
        Tile::Door => (
            Style::default().fg(Color::Yellow).bg(Color::Rgb(139, 69, 19)), // Brown door
            '+'
        ),
        Tile::Grass => (
            Style::default().fg(Color::Green),
            '"'
        ),
        Tile::Tree => (
            Style::default().fg(Color::Green).bg(Color::Rgb(34, 139, 34)), // Forest green background
            'T'
        ),
        Tile::Mountain => (
            Style::default().fg(Color::White).bg(Color::Rgb(105, 105, 105)), // Dim gray background
            '^'
        ),
        Tile::Water => (
            Style::default().fg(Color::Cyan).bg(Color::Blue),
            '~'
        ),
        Tile::Road => (
            Style::default().fg(Color::Yellow).bg(Color::Rgb(139, 69, 19)), // Saddle brown background
            '+'
        ),
        Tile::Village => (
            Style::default().fg(Color::Magenta).bg(Color::Rgb(255, 215, 0)), // Gold background
            'V'
        ),
        Tile::DungeonEntrance => (
            Style::default().fg(Color::Red).bg(Color::Black),
            'D'
        ),
        Tile::DungeonExit => (
            Style::default().fg(Color::Cyan).bg(Color::Black),
            '<'
        ),
        Tile::CaveFloor => (
            Style::default().fg(Color::Rgb(139, 119, 101)), // Sandy brown
            '.'
        ),
        Tile::CaveWall => (
            Style::default().fg(Color::Rgb(105, 105, 105)).bg(Color::Rgb(64, 64, 64)), // Dim gray
            '#'
        ),
        Tile::Corridor => (
            Style::default().fg(Color::Rgb(169, 169, 169)), // Dark gray
            '.'
        ),
    }
}

fn render_inventory(frame: &mut Frame, _app: &App, area: Rect) {
    let inventory_block = Block::default()
        .borders(Borders::ALL)
        .title("Inventory")
        .style(Style::default());

    let inventory_text = "Your inventory is empty.\n\nPress 'g' to return to game.";
    
    let inventory = Paragraph::new(Text::styled(
        inventory_text,
        Style::default().fg(Color::Yellow),
    ))
    .block(inventory_block);

    frame.render_widget(inventory, area);
}

fn render_exit_screen(frame: &mut Frame, _app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    
    let popup_block = Block::default()
        .title("Quit Game")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));

    let exit_text = Text::styled(
        "Are you sure you want to quit? (y/n)",
        Style::default().fg(Color::Red),
    );
    
    let exit_paragraph = Paragraph::new(exit_text)
        .block(popup_block)
        .wrap(Wrap { trim: false });

    let popup_area = centered_rect(50, 20, area);
    frame.render_widget(exit_paragraph, popup_area);
}

/// Helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Helper function to wrap text to a specified width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    for word in words {
        // If adding this word would exceed the width, start a new line
        if !current_line.is_empty() && current_line.len() + 1 + word.len() > width {
            lines.push(current_line);
            current_line = word.to_string();
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }
    
    // Add the last line if it's not empty
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    // Return at least one line (empty if no words)
    if lines.is_empty() {
        lines.push(String::new());
    }
    
    lines
}

fn render_chat_input_bar(frame: &mut Frame, app: &App, area: Rect) {
    // Wrap the chat input text to fit the available width
    let available_width = (area.width.saturating_sub(4)) as usize; // Account for borders and prefix
    let prefix = "> ";
    let wrapped_lines = wrap_text(&app.chat_input, available_width.saturating_sub(prefix.len()));
    
    // Create text with proper wrapping - display from top to bottom
    let mut lines = Vec::new();
    
    if wrapped_lines.is_empty() {
        lines.push(Line::from(Span::styled(
            prefix.to_string(),
            Style::default().fg(Color::Green),
        )));
    } else {
        for (i, line) in wrapped_lines.iter().enumerate() {
            let display_line = if i == 0 {
                format!("{}{}", prefix, line)
            } else {
                format!("  {}", line) // Indent continuation lines
            };
            
            lines.push(Line::from(Span::styled(
                display_line,
                Style::default().fg(Color::Green),
            )));
        }
    }
    
    let chat_input_widget = Paragraph::new(Text::from(lines))
        .block(Block::default()
            .borders(Borders::ALL)
            .title("💬 Chat (Press Enter to send, Esc to cancel)")
            .title_style(Style::default().fg(Color::Yellow)));
    
    frame.render_widget(chat_input_widget, area);
}

/// Apply brightness to a style for the lighting system
fn apply_brightness_to_style(base_style: Style, brightness: f32) -> Style {
    // Extract the original foreground color
    let original_color = base_style.fg.unwrap_or(Color::White);
    
    // Apply brightness by modifying the color
    let modified_color = match original_color {
        Color::Rgb(r, g, b) => {
            let new_r = ((r as f32 * brightness) as u8).min(255);
            let new_g = ((g as f32 * brightness) as u8).min(255);
            let new_b = ((b as f32 * brightness) as u8).min(255);
            Color::Rgb(new_r, new_g, new_b)
        }
        Color::Reset => Color::Reset,
        Color::Black => Color::Black,
        Color::Red => {
            if brightness > 0.8 { Color::Red }
            else if brightness > 0.5 { Color::from_u32(0x800000) } // Dark red
            else { Color::from_u32(0x400000) } // Very dark red
        }
        Color::Green => {
            if brightness > 0.8 { Color::Green }
            else if brightness > 0.5 { Color::from_u32(0x008000) } // Dark green
            else { Color::from_u32(0x004000) } // Very dark green
        }
        Color::Yellow => {
            if brightness > 0.8 { Color::Yellow }
            else if brightness > 0.5 { Color::from_u32(0x808000) } // Dark yellow
            else { Color::from_u32(0x404000) } // Very dark yellow
        }
        Color::Blue => {
            if brightness > 0.8 { Color::Blue }
            else if brightness > 0.5 { Color::from_u32(0x000080) } // Dark blue
            else { Color::from_u32(0x000040) } // Very dark blue
        }
        Color::Magenta => {
            if brightness > 0.8 { Color::Magenta }
            else if brightness > 0.5 { Color::from_u32(0x800080) } // Dark magenta
            else { Color::from_u32(0x400040) } // Very dark magenta
        }
        Color::Cyan => {
            if brightness > 0.8 { Color::Cyan }
            else if brightness > 0.5 { Color::from_u32(0x008080) } // Dark cyan
            else { Color::from_u32(0x004040) } // Very dark cyan
        }
        Color::White => {
            if brightness > 0.8 { Color::White }
            else if brightness > 0.5 { Color::Gray }
            else { Color::DarkGray }
        }
        Color::Gray => {
            if brightness > 0.5 { Color::Gray }
            else { Color::DarkGray }
        }
        Color::DarkGray => Color::DarkGray,
        _ => {
            // For other colors, try to dim them
            if brightness > 0.5 { original_color }
            else { Color::DarkGray }
        }
    };
    
    // Return the style with the modified color
    Style::default().fg(modified_color).bg(base_style.bg.unwrap_or(Color::Reset))
}