use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen, MapType, Tile};

pub fn ui(frame: &mut Frame, app: &App) {
    // Create the layout sections: Status bar, Game area, Message log
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Status bar
            Constraint::Min(1),     // Game area
            Constraint::Length(5),  // Message log
        ])
        .split(frame.area());

    // Status bar showing player stats and current screen
    let status_text = format!(
        "HP: {}/{} | Turn: {} | Map: {} | Controls: HJKL/Arrows (move), E (enter dungeon), X (exit dungeon), I (inventory), Q (quit)",
        app.player.hp, 
        app.player.max_hp, 
        app.turn_count, 
        match app.current_map_type {
            MapType::Overworld => "Overworld",
            MapType::Dungeon => "Dungeon",
        }
    );
    
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

    // Game area - render based on current screen
    match app.current_screen {
        CurrentScreen::Game => render_game_map(frame, app, chunks[1]),
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
    let mut lines = Vec::<Line>::new();
    
    for y in 0..app.game_map.height {
        let mut spans = Vec::<Span>::new();
        
        for x in 0..app.game_map.width {
            if x == app.player.x && y == app.player.y {
                // Player character with bright yellow foreground and dark background
                spans.push(Span::styled(
                    app.player.symbol.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::DarkGray)
                ));
            } else if let Some(tile) = app.game_map.tiles.get(&(x, y)) {
                let (style, character) = get_tile_style_and_char(*tile);
                spans.push(Span::styled(character.to_string(), style));
            } else {
                spans.push(Span::styled(" ".to_string(), Style::default()));
            }
        }
        lines.push(Line::from(spans));
    }

    let title = match app.current_map_type {
        MapType::Overworld => "🌍 Overworld",
        MapType::Dungeon => "🏰 Dungeon",
    };

    let game_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default());

    let game_area = Paragraph::new(Text::from(lines))
        .block(game_block);

    frame.render_widget(game_area, area);
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

fn render_inventory(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_exit_screen(frame: &mut Frame, app: &App, area: Rect) {
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