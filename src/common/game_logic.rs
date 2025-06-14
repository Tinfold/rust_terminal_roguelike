// Shared game logic to reduce duplication between client and server
use std::collections::HashMap;
use super::protocol::{NetworkGameMap, coord_to_string, string_to_coord};
use super::constants::GameConstants;
use super::terrain::TerrainGenerator;
use super::chunk::ChunkManager;

// Re-export common types that both client and server need
pub use super::terrain::{Tile, GameMap};
pub use super::chunk::{ChunkManager as GameChunkManager, ChunkCoord};

#[derive(Debug, Clone)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub symbol: char,
    pub dungeon_entrance_pos: Option<(i32, i32)>, // Position of the dungeon entrance they came from
}

pub struct GameLogic;

impl GameLogic {
    /// Validates if movement to a tile is allowed
    pub fn is_movement_valid(tile: Tile) -> bool {
        matches!(tile, 
            Tile::Floor | Tile::Grass | Tile::Road | 
            Tile::Tree | Tile::Village | Tile::DungeonEntrance | Tile::Door | Tile::DungeonExit
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
            Tile::DungeonExit => Some("You are at the dungeon entrance/exit. Press 'x' to exit to the overworld.".to_string()),
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
        // Use the sophisticated terrain generator from the terrain module
        let width = GameConstants::DUNGEON_WIDTH;
        let height = GameConstants::DUNGEON_HEIGHT;
        
        TerrainGenerator::generate_dungeon(width, height)
    }

    /// Generate a dungeon map based on entrance position for uniqueness
    pub fn generate_dungeon_map_for_entrance(entrance_x: i32, entrance_y: i32) -> GameMap {
        let width = GameConstants::DUNGEON_WIDTH;
        let height = GameConstants::DUNGEON_HEIGHT;
        
        // Generate a unique seed based on entrance position
        let seed = Self::generate_dungeon_seed(entrance_x, entrance_y);
        
        TerrainGenerator::generate_dungeon_with_seed(width, height, seed)
    }

    /// Generate a unique seed for a dungeon based on its entrance position
    pub fn generate_dungeon_seed(entrance_x: i32, entrance_y: i32) -> u32 {
        // Use entrance coordinates to create a deterministic but unique seed
        let mut seed = 0x9e3779b9u32; // A good base seed (golden ratio * 2^32)
        seed = seed.wrapping_add(entrance_x as u32).wrapping_mul(0x85ebca6b);
        seed = seed.wrapping_add(entrance_y as u32).wrapping_mul(0xc2b2ae35);
        seed = seed ^ (seed >> 16);
        seed = seed.wrapping_mul(0x85ebca6b);
        seed = seed ^ (seed >> 13);
        seed = seed.wrapping_mul(0xc2b2ae35);
        seed = seed ^ (seed >> 16);
        seed
    }

    /// Common logic for exiting to overworld - generates the overworld map
    pub fn generate_overworld_map() -> GameMap {
        // Use the sophisticated terrain generator from the terrain module
        let width = GameConstants::OVERWORLD_WIDTH;
        let height = GameConstants::OVERWORLD_HEIGHT;
        
        TerrainGenerator::generate_overworld(width, height)
    }

    /// Generate a dungeon map with a specific seed for consistency
    pub fn generate_dungeon_map_with_seed(seed: u32) -> GameMap {
        // Use the seed to ensure consistent dungeon generation
        // Different seeds could generate different dungeons for different entrances
        let width = GameConstants::DUNGEON_WIDTH;
        let height = GameConstants::DUNGEON_HEIGHT;
        
        // For now, we use the standard generator but could enhance it with seed support
        let _ = seed; // Acknowledge the parameter for future use
        TerrainGenerator::generate_dungeon(width, height)
    }

    /// Get default dungeon spawn position - now finds a safe floor tile
    pub fn get_dungeon_spawn_position() -> (i32, i32) {
        (GameConstants::DUNGEON_SPAWN_X, GameConstants::DUNGEON_SPAWN_Y)
    }

    /// Get a safe spawn position in a given dungeon map
    pub fn get_safe_dungeon_spawn_position(dungeon_map: &GameMap) -> (i32, i32) {
        // First, look for a DungeonExit tile - this is the intended spawn position
        for y in 1..dungeon_map.height - 1 {
            for x in 1..dungeon_map.width - 1 {
                if let Some(tile) = dungeon_map.tiles.get(&(x, y)) {
                    if *tile == Tile::DungeonExit {
                        return (x, y);
                    }
                }
            }
        }
        
        // If no dungeon exit found, try the default spawn position
        let default_pos = (GameConstants::DUNGEON_SPAWN_X, GameConstants::DUNGEON_SPAWN_Y);
        if let Some(tile) = dungeon_map.tiles.get(&default_pos) {
            if *tile == Tile::Floor {
                return default_pos;
            }
        }

        // If default position is not safe, find the first floor tile
        for y in 1..dungeon_map.height - 1 {
            for x in 1..dungeon_map.width - 1 {
                if let Some(tile) = dungeon_map.tiles.get(&(x, y)) {
                    if *tile == Tile::Floor {
                        return (x, y);
                    }
                }
            }
        }

        // Fallback to default position (should not happen with proper generation)
        default_pos
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

    /// Check if current position has a dungeon exit
    pub fn is_at_dungeon_exit(game_map: &GameMap, x: i32, y: i32) -> bool {
        game_map.tiles.get(&(x, y)) == Some(&Tile::DungeonExit)
    }

    /// Limit messages to a maximum count
    pub fn limit_messages(messages: &mut Vec<String>, max_count: usize) {
        while messages.len() > max_count {
            messages.remove(0);
        }
    }

    /// Create a new chunk manager with infinite terrain
    pub fn create_chunk_manager(seed: u32) -> GameChunkManager {
        GameChunkManager::new(seed)
    }

    /// Check if current position has a dungeon entrance (chunk manager version)
    pub fn is_at_chunk_dungeon_entrance(chunk_manager: &mut GameChunkManager, x: i32, y: i32) -> bool {
        chunk_manager.get_tile(x, y) == Some(Tile::DungeonEntrance)
    }

    /// Get tiles in area from chunk manager for rendering
    pub fn get_viewport_tiles(chunk_manager: &mut GameChunkManager, center_x: i32, center_y: i32, width: i32, height: i32) -> HashMap<(i32, i32), Tile> {
        let min_x = center_x - width / 2;
        let min_y = center_y - height / 2;
        let max_x = center_x + width / 2;
        let max_y = center_y + height / 2;
        
        chunk_manager.get_tiles_in_area(min_x, min_y, max_x, max_y)
    }
}

/// Trait for common player operations
pub trait PlayerOperations {
    fn get_position(&self) -> (i32, i32);
    fn set_position(&mut self, x: i32, y: i32);
    fn get_hp(&self) -> i32;
    fn set_hp(&mut self, hp: i32);
}

// Implement for common Player
impl PlayerOperations for Player {
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

// Implement for NetworkPlayer
impl PlayerOperations for super::protocol::NetworkPlayer {
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
