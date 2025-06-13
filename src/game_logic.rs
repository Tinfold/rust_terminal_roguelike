// Shared game logic to reduce duplication between client and server
use std::collections::HashMap;
use crate::protocol::{NetworkGameMap, coord_to_string, string_to_coord};
use crate::app::{GameMap, Tile};
use crate::terrain::TerrainGenerator;
use crate::constants::GameConstants;

pub struct GameLogic;

impl GameLogic {
    /// Validates if movement to a tile is allowed
    pub fn is_movement_valid(tile: Tile) -> bool {
        matches!(tile, 
            Tile::Floor | Tile::Grass | Tile::Road | 
            Tile::Tree | Tile::Village | Tile::DungeonEntrance
        )
    }

    /// Gets the message for blocked movement
    pub fn get_blocked_movement_message(tile: Tile) -> String {
        match tile {
            Tile::Wall => "You can't move through a wall.".to_string(),
            Tile::Mountain => "You can't move through a mountain.".to_string(),
            Tile::Water => "You can't swim across the water.".to_string(),
            _ => "You can't move there.".to_string(),
        }
    }

    /// Gets flavor text for moving to certain tiles
    pub fn get_tile_interaction_message(tile: Tile) -> Option<String> {
        match tile {
            Tile::Tree => Some("You push through the thick forest.".to_string()),
            Tile::Village => Some("You visit the village. The locals greet you warmly.".to_string()),
            Tile::DungeonEntrance => Some("You stand before a dark dungeon entrance. Press 'e' to enter.".to_string()),
            _ => None,
        }
    }

    /// Converts a GameMap to NetworkGameMap
    pub fn game_map_to_network(game_map: &GameMap) -> NetworkGameMap {
        let network_tiles: HashMap<String, Tile> = game_map.tiles
            .iter()
            .map(|((x, y), tile)| (coord_to_string(*x, *y), *tile))
            .collect();

        NetworkGameMap {
            width: game_map.width,
            height: game_map.height,
            tiles: network_tiles,
        }
    }

    /// Converts a NetworkGameMap to GameMap
    pub fn network_map_to_game(network_map: &NetworkGameMap) -> GameMap {
        let mut tiles = HashMap::new();
        for (coord_str, tile) in &network_map.tiles {
            if let Some((x, y)) = string_to_coord(coord_str) {
                tiles.insert((x, y), *tile);
            }
        }
        
        GameMap {
            width: network_map.width,
            height: network_map.height,
            tiles,
        }
    }

    /// Common logic for entering a dungeon - generates the dungeon map
    pub fn generate_dungeon_map() -> GameMap {
        TerrainGenerator::generate_dungeon(GameConstants::DUNGEON_WIDTH, GameConstants::DUNGEON_HEIGHT)
    }

    /// Common logic for exiting to overworld - generates the overworld map
    pub fn generate_overworld_map() -> GameMap {
        TerrainGenerator::generate_overworld(GameConstants::OVERWORLD_WIDTH, GameConstants::OVERWORLD_HEIGHT)
    }

    /// Get default dungeon spawn position
    pub fn get_dungeon_spawn_position() -> (i32, i32) {
        (GameConstants::DUNGEON_SPAWN_X, GameConstants::DUNGEON_SPAWN_Y)
    }

    /// Get default overworld spawn position
    pub fn get_overworld_spawn_position() -> (i32, i32) {
        (GameConstants::OVERWORLD_SPAWN_X, GameConstants::OVERWORLD_SPAWN_Y)
    }

    /// Check if current position has a dungeon entrance
    pub fn is_at_dungeon_entrance(game_map: &GameMap, x: i32, y: i32) -> bool {
        game_map.tiles.get(&(x, y)) == Some(&Tile::DungeonEntrance)
    }

    /// Check if current position has a dungeon entrance (network version)
    pub fn is_at_network_dungeon_entrance(game_map: &NetworkGameMap, x: i32, y: i32) -> bool {
        game_map.get_tile(x, y) == Some(&Tile::DungeonEntrance)
    }

    /// Limit messages to a maximum count
    pub fn limit_messages(messages: &mut Vec<String>, max_count: usize) {
        while messages.len() > max_count {
            messages.remove(0);
        }
    }
}

/// Trait for common player operations
pub trait PlayerOperations {
    fn get_position(&self) -> (i32, i32);
    fn set_position(&mut self, x: i32, y: i32);
    fn get_hp(&self) -> i32;
    fn set_hp(&mut self, hp: i32);
}

// Implement for client Player
impl PlayerOperations for crate::app::Player {
    fn get_position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    fn get_hp(&self) -> i32 {
        self.hp
    }

    fn set_hp(&mut self, hp: i32) {
        self.hp = hp;
    }
}

// Implement for server NetworkPlayer
impl PlayerOperations for crate::protocol::NetworkPlayer {
    fn get_position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    fn get_hp(&self) -> i32 {
        self.hp
    }

    fn set_hp(&mut self, hp: i32) {
        self.hp = hp;
    }
}
