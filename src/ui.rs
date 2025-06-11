use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen, Tile};

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
        "HP: {}/{} | Turn: {} | Screen: {:?}",
        app.player.hp, app.player.max_hp, app.turn_count, app.current_screen
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
    let mut map_text = String::new();
    
    for y in 0..app.game_map.height {
        for x in 0..app.game_map.width {
            if x == app.player.x && y == app.player.y {
                map_text.push(app.player.symbol);
            } else if let Some(tile) = app.game_map.tiles.get(&(x, y)) {
                map_text.push(tile.to_char());
            } else {
                map_text.push(' ');
            }
        }
        map_text.push('\n');
    }

    let game_block = Block::default()
        .borders(Borders::ALL)
        .title("Dungeon")
        .style(Style::default());

    let game_area = Paragraph::new(Text::styled(
        map_text,
        Style::default().fg(Color::White),
    ))
    .block(game_block);

    frame.render_widget(game_area, area);
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