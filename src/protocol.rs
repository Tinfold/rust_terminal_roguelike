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
    pub current_map_type: NetworkMapType,
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
    pub tiles: HashMap<String, NetworkTile>, // Changed from (i32, i32) to String
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkTile {
    Floor,
    Wall,
    Empty,
    Grass,
    Tree,
    Mountain,
    Water,
    Road,
    Village,
    DungeonEntrance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMapType {
    Overworld,
    Dungeon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkCurrentScreen {
    Game,
    Inventory,
    Exiting,
}

impl From<crate::app::Tile> for NetworkTile {
    fn from(tile: crate::app::Tile) -> Self {
        match tile {
            crate::app::Tile::Floor => NetworkTile::Floor,
            crate::app::Tile::Wall => NetworkTile::Wall,
            crate::app::Tile::Empty => NetworkTile::Empty,
            crate::app::Tile::Grass => NetworkTile::Grass,
            crate::app::Tile::Tree => NetworkTile::Tree,
            crate::app::Tile::Mountain => NetworkTile::Mountain,
            crate::app::Tile::Water => NetworkTile::Water,
            crate::app::Tile::Road => NetworkTile::Road,
            crate::app::Tile::Village => NetworkTile::Village,
            crate::app::Tile::DungeonEntrance => NetworkTile::DungeonEntrance,
        }
    }
}

impl From<NetworkTile> for crate::app::Tile {
    fn from(tile: NetworkTile) -> Self {
        match tile {
            NetworkTile::Floor => crate::app::Tile::Floor,
            NetworkTile::Wall => crate::app::Tile::Wall,
            NetworkTile::Empty => crate::app::Tile::Empty,
            NetworkTile::Grass => crate::app::Tile::Grass,
            NetworkTile::Tree => crate::app::Tile::Tree,
            NetworkTile::Mountain => crate::app::Tile::Mountain,
            NetworkTile::Water => crate::app::Tile::Water,
            NetworkTile::Road => crate::app::Tile::Road,
            NetworkTile::Village => crate::app::Tile::Village,
            NetworkTile::DungeonEntrance => crate::app::Tile::DungeonEntrance,
        }
    }
}

impl From<crate::app::MapType> for NetworkMapType {
    fn from(map_type: crate::app::MapType) -> Self {
        match map_type {
            crate::app::MapType::Overworld => NetworkMapType::Overworld,
            crate::app::MapType::Dungeon => NetworkMapType::Dungeon,
        }
    }
}

impl From<NetworkMapType> for crate::app::MapType {
    fn from(map_type: NetworkMapType) -> Self {
        match map_type {
            NetworkMapType::Overworld => crate::app::MapType::Overworld,
            NetworkMapType::Dungeon => crate::app::MapType::Dungeon,
        }
    }
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
    pub fn get_tile(&self, x: i32, y: i32) -> Option<&NetworkTile> {
        self.tiles.get(&coord_to_string(x, y))
    }
    
    pub fn set_tile(&mut self, x: i32, y: i32, tile: NetworkTile) {
        self.tiles.insert(coord_to_string(x, y), tile);
    }
}
