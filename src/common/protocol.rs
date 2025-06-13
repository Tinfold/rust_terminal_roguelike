use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::game_logic::Tile;

pub type PlayerId = String;

// Define the enums that both client and server need
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MapType {
    Overworld,
    Dungeon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CurrentScreen {
    MainMenu,
    Game,
    Inventory,
    Chat,
    Exiting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { player_name: String },
    Move { dx: i32, dy: i32 },
    RequestChunks { chunks: Vec<(i32, i32)> }, // Request specific chunk coordinates
    EnterDungeon,
    ExitDungeon,
    OpenInventory,
    CloseInventory,
    Chat { message: String },
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Connected { player_id: PlayerId },
    GameState { state: GameState },
    ChunkData { chunks: Vec<ChunkData> }, // Send chunk data to clients
    PlayerMoved { player_id: PlayerId, x: i32, y: i32 },
    PlayerJoined { player_id: PlayerId, player: NetworkPlayer },
    PlayerLeft { player_id: PlayerId },
    Error { message: String },
    Message { text: String },
    ChatMessage { player_name: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub players: HashMap<PlayerId, NetworkPlayer>,
    pub turn_count: u32,
    // Chunks are sent separately via ChunkData messages
    // Note: current_map_type is now per-player
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
    pub color: (u8, u8, u8), // RGB color tuple for this player
    pub current_map_type: MapType, // Each player can be in a different map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkGameMap {
    pub width: i32,
    pub height: i32,
    pub tiles: HashMap<String, Tile>, // Using Tile directly now
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub tiles: HashMap<String, Tile>, // Local coordinates as string keys (e.g., "0,0" to "31,31")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkCurrentScreen {
    Game,
    Inventory,
    Chat,
    Exiting,
}

impl From<CurrentScreen> for NetworkCurrentScreen {
    fn from(screen: CurrentScreen) -> Self {
        match screen {
            CurrentScreen::MainMenu => NetworkCurrentScreen::Game, // Map MainMenu to Game for network
            CurrentScreen::Game => NetworkCurrentScreen::Game,
            CurrentScreen::Inventory => NetworkCurrentScreen::Inventory,
            CurrentScreen::Chat => NetworkCurrentScreen::Chat,
            CurrentScreen::Exiting => NetworkCurrentScreen::Exiting,
        }
    }
}

impl From<NetworkCurrentScreen> for CurrentScreen {
    fn from(screen: NetworkCurrentScreen) -> Self {
        match screen {
            NetworkCurrentScreen::Game => CurrentScreen::Game,
            NetworkCurrentScreen::Inventory => CurrentScreen::Inventory,
            NetworkCurrentScreen::Chat => CurrentScreen::Chat,
            NetworkCurrentScreen::Exiting => CurrentScreen::Exiting,
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
    pub fn get_tile(&self, x: i32, y: i32) -> Option<&Tile> {
        self.tiles.get(&coord_to_string(x, y))
    }
}
