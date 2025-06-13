use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type PlayerId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { player_name: String },
    Move { dx: i32, dy: i32 },
    EnterDungeon,
    ExitDungeon,
    OpenInventory,
    CloseInventory,
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Connected { player_id: PlayerId },
    GameState { state: GameState },
    PlayerMoved { player_id: PlayerId, x: i32, y: i32 },
    PlayerJoined { player_id: PlayerId, player: NetworkPlayer },
    PlayerLeft { player_id: PlayerId },
    Error { message: String },
    Message { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub players: HashMap<PlayerId, NetworkPlayer>,
    pub game_map: NetworkGameMap,
    pub current_map_type: crate::app::MapType,
    pub turn_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPlayer {
    pub id: PlayerId,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub symbol: char,
    pub current_screen: NetworkCurrentScreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkGameMap {
    pub width: i32,
    pub height: i32,
    pub tiles: HashMap<String, crate::app::Tile>, // Using Tile directly now
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkCurrentScreen {
    Game,
    Inventory,
    Exiting,
}

impl From<crate::app::CurrentScreen> for NetworkCurrentScreen {
    fn from(screen: crate::app::CurrentScreen) -> Self {
        match screen {
            crate::app::CurrentScreen::MainMenu => NetworkCurrentScreen::Game, // Map MainMenu to Game for network
            crate::app::CurrentScreen::Game => NetworkCurrentScreen::Game,
            crate::app::CurrentScreen::Inventory => NetworkCurrentScreen::Inventory,
            crate::app::CurrentScreen::Exiting => NetworkCurrentScreen::Exiting,
        }
    }
}

impl From<NetworkCurrentScreen> for crate::app::CurrentScreen {
    fn from(screen: NetworkCurrentScreen) -> Self {
        match screen {
            NetworkCurrentScreen::Game => crate::app::CurrentScreen::Game,
            NetworkCurrentScreen::Inventory => crate::app::CurrentScreen::Inventory,
            NetworkCurrentScreen::Exiting => crate::app::CurrentScreen::Exiting,
        }
    }
}

// Helper functions for coordinate conversion
pub fn coord_to_string(x: i32, y: i32) -> String {
    format!("{}_{}", x, y)
}

pub fn string_to_coord(s: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() == 2 {
        if let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
            return Some((x, y));
        }
    }
    None
}

impl NetworkGameMap {
    pub fn get_tile(&self, x: i32, y: i32) -> Option<&crate::app::Tile> {
        self.tiles.get(&coord_to_string(x, y))
    }
}
