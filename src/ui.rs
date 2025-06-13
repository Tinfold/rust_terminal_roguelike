use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen, MapType, Tile, GameMode};

pub fn ui(frame: &mut Frame, app: &App) {
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
    let menu_items = vec![
        "Single Player",
        "Multiplayer",
        "Quit",
    ];

    let mut menu_list_items = Vec::<ListItem>::new();
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

    let menu_list = List::new(menu_list_items)
        .block(Block::default().borders(Borders::ALL).title("Select Option (↑/↓ to select, Enter to confirm)"));

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

fn render_game_ui(frame: &mut Frame, app: &App) {
    // Create the layout sections: Status bar, Game area, Message log
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Status bar
            Constraint::Min(20),    // Game area (minimum height)
            Constraint::Length(5),  // Message log
        ])
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

    // Message log
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

fn render_game_map(frame: &mut Frame, app: &App, area: Rect) {
    // Calculate the viewport size (accounting for borders)
    let viewport_width = (area.width.saturating_sub(2)) as i32; // Subtract 2 for borders
    let viewport_height = (area.height.saturating_sub(2)) as i32; // Subtract 2 for borders
    
    // Ensure minimum viewport size and make width wider to utilize terminal space better
    let viewport_width = viewport_width.max(60); // Increased minimum width
    let viewport_height = viewport_height.max(20); // Increased minimum height
    
    // Calculate camera position to center on player
    let camera_x = app.player.x - viewport_width / 2;
    let camera_y = app.player.y - viewport_height / 2;
    
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
            } else if let Some(other_player) = app.other_players.values().find(|p| p.x == world_x && p.y == world_y) {
                // Other players in multiplayer mode
                spans.push(Span::styled(
                    other_player.symbol.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::DarkGray)
                ));
            } else if let Some(tile) = app.game_map.tiles.get(&(world_x, world_y)) {
                let (style, character) = get_tile_style_and_char(*tile);
                spans.push(Span::styled(character.to_string(), style));
            } else {
                // Out of bounds or empty space - show void
                spans.push(Span::styled(" ".to_string(), Style::default().bg(Color::Black)));
            }
        }
        lines.push(Line::from(spans));
    }

    let title = match app.current_map_type {
        MapType::Overworld => {
            if app.game_mode == GameMode::MultiPlayer {
                format!("🌍 Overworld (Players: {})", app.other_players.len() + 1)
            } else {
                "🌍 Overworld".to_string()
            }
        },
        MapType::Dungeon => {
            if app.game_mode == GameMode::MultiPlayer {
                format!("🏰 Dungeon (Players: {})", app.other_players.len() + 1)
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

fn render_chat_screen(frame: &mut Frame, app: &App) {
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

    // Chat messages
    let mut chat_items = Vec::<ListItem>::new();
    for (player_name, message) in app.chat_messages.iter().rev().take(15) { // Show last 15 messages
        chat_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{}: ", player_name),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                message.clone(),
                Style::default().fg(Color::White),
            ),
        ])));
    }
    chat_items.reverse(); // Show messages in chronological order

    let chat_list = List::new(chat_items)
        .block(Block::default().borders(Borders::ALL).title("Chat Messages"));
    frame.render_widget(chat_list, chunks[1]);

    // Input box
    let input_text = format!("> {}", app.chat_input);
    let input = Paragraph::new(Text::styled(
        input_text,
        Style::default().fg(Color::Green),
    ))
    .block(Block::default().borders(Borders::ALL).title("Type your message"));
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
    // Chat widget for multiplayer mode
    let mut chat_items = Vec::<ListItem>::new();
    
    // Show the last 10 chat messages in the widget
    for (player_name, message) in app.chat_messages.iter().rev().take(10) {
        // Truncate long messages to fit in the widget
        let truncated_message = if message.len() > 25 {
            format!("{}...", &message[..22])
        } else {
            message.clone()
        };
        
        chat_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{}: ", player_name),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                truncated_message,
                Style::default().fg(Color::White),
            ),
        ])));
    }
    chat_items.reverse(); // Show messages in chronological order

    let chat_title = format!("💬 Chat ({})", app.chat_messages.len());
    let chat_list = List::new(chat_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(chat_title)
            .title_style(Style::default().fg(Color::Yellow)));
    
    frame.render_widget(chat_list, area);
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