// Shared game logic to reduce duplication between client and server
use std::collections::HashMap;
use super::protocol::{NetworkGameMap, coord_to_string, string_to_coord};
use super::constants::GameConstants;
use super::terrain::TerrainGenerator;
use super::chunk::ChunkManager;
use super::dungeon::DungeonGenerator;

// Re-export common types that both client and server need
pub use super::terrain::{Tile, GameMap, Room};
pub use super::chunk::{ChunkManager as GameChunkManager, ChunkCoord};

#[derive(Debug, Clone)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub symbol: char,
    pub dungeon_entrance_pos: Option<(i32, i32)>, // Position of the dungeon entrance they came from
    // Exploration tracking for dungeons
    pub opened_doors: std::collections::HashSet<(i32, i32)>, // Positions of doors that have been opened
    pub explored_rooms: std::collections::HashSet<u32>, // IDs of rooms that have been explored
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
            rooms: Vec::new(), // Network maps don't include room data currently
            room_positions: HashMap::new(),
            visible_tiles: HashMap::new(),
            explored_tiles: HashMap::new(),
            illuminated_areas: HashMap::new(),
        }
    }

    /// Common logic for entering a dungeon - generates the dungeon map
    pub fn generate_dungeon_map() -> GameMap {
        // Use the sophisticated dungeon generator from the dungeon module
        let width = GameConstants::DUNGEON_WIDTH;
        let height = GameConstants::DUNGEON_HEIGHT;
        
        DungeonGenerator::generate_dungeon(width, height)
    }

    /// Generate a dungeon map based on entrance position for uniqueness
    pub fn generate_dungeon_map_for_entrance(entrance_x: i32, entrance_y: i32) -> GameMap {
        DungeonGenerator::generate_dungeon_for_entrance(entrance_x, entrance_y)
    }

    /// Generate a unique seed for a dungeon based on its entrance position
    pub fn generate_dungeon_seed(entrance_x: i32, entrance_y: i32) -> u32 {
        DungeonGenerator::generate_dungeon_seed(entrance_x, entrance_y)
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
        let width = GameConstants::DUNGEON_WIDTH;
        let height = GameConstants::DUNGEON_HEIGHT;
        
        DungeonGenerator::generate_dungeon_with_seed(width, height, seed)
    }

    /// Get default dungeon spawn position - now finds a safe floor tile
    pub fn get_dungeon_spawn_position() -> (i32, i32) {
        DungeonGenerator::get_default_spawn_position()
    }

    /// Get a safe spawn position in a given dungeon map
    pub fn get_safe_dungeon_spawn_position(dungeon_map: &GameMap) -> (i32, i32) {
        DungeonGenerator::get_safe_spawn_position(dungeon_map)
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

    /// Check if a tile should be visible based on room exploration
    pub fn is_tile_visible(game_map: &GameMap, player: &Player, x: i32, y: i32) -> bool {
        // In overworld, all tiles are always visible
        if game_map.rooms.is_empty() {
            return true;
        }
        
        // Check if position is in a room
        if let Some(&room_id) = game_map.room_positions.get(&(x, y)) {
            // Tile is visible if the room has been explored
            return player.explored_rooms.contains(&room_id);
        }
        
        // Check if position is a corridor (not in any room)
        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
            if tile == Tile::Floor {
                // For corridors, check if any adjacent explored room makes it visible
                for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                        if player.explored_rooms.contains(&room_id) {
                            return true;
                        }
                    }
                }
                
                // Also check if connected to the corridor network through opened doors
                if Self::is_corridor_connected_to_explored_area(game_map, player, x, y) {
                    return true;
                }
            }
        }
        
        // Doors are visible if they connect to an explored room
        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
            if tile == Tile::Door {
                // Check if door has been opened
                if player.opened_doors.contains(&(x, y)) {
                    return true;
                }
                
                // Check if door is adjacent to an explored room
                for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                        if player.explored_rooms.contains(&room_id) {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Dungeon exit is always visible
        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
            if tile == Tile::DungeonExit {
                return true;
            }
        }
        
        false
    }

    /// Handle opening a door and revealing connected rooms and corridors
    pub fn open_door(game_map: &GameMap, player: &mut Player, x: i32, y: i32) -> bool {
        // Check if position has a door
        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
            if tile == Tile::Door {
                // Mark door as opened
                player.opened_doors.insert((x, y));
                
                // Reveal rooms connected by this door
                for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                        player.explored_rooms.insert(room_id);
                    }
                }
                
                // Also reveal connected corridor networks
                Self::reveal_connected_corridors(game_map, player, x, y);
                
                return true;
            }
        }
        false
    }

    /// Reveal corridor network connected to a position using flood fill
    fn reveal_connected_corridors(game_map: &GameMap, player: &mut Player, start_x: i32, start_y: i32) {
        let mut stack = vec![(start_x, start_y)];
        let mut visited = std::collections::HashSet::new();
        
        while let Some((x, y)) = stack.pop() {
            if visited.contains(&(x, y)) {
                continue;
            }
            visited.insert((x, y));
            
            // Check all adjacent tiles
            for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x + dx;
                let ny = y + dy;
                
                // Skip if already visited
                if visited.contains(&(nx, ny)) {
                    continue;
                }
                
                // Check if this position has a walkable tile
                if let Some(&tile) = game_map.tiles.get(&(nx, ny)) {
                    match tile {
                        Tile::Floor => {
                            // If it's a corridor (not in a room), continue exploring
                            if game_map.room_positions.get(&(nx, ny)).is_none() {
                                stack.push((nx, ny));
                            }
                        },
                        Tile::Door => {
                            // If it's a door that connects to an explored room or corridor, continue exploring
                            let connects_to_explored = Self::door_connects_to_explored_area(game_map, player, nx, ny);
                            if connects_to_explored || player.opened_doors.contains(&(nx, ny)) {
                                player.opened_doors.insert((nx, ny));
                                stack.push((nx, ny));
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    
    /// Check if a door connects to an already explored area
    fn door_connects_to_explored_area(game_map: &GameMap, player: &Player, door_x: i32, door_y: i32) -> bool {
        for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = door_x + dx;
            let ny = door_y + dy;
            
            // Check if adjacent to an explored room
            if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                if player.explored_rooms.contains(&room_id) {
                    return true;
                }
            }
            
            // Check if adjacent to a corridor that should be visible
            if let Some(&tile) = game_map.tiles.get(&(nx, ny)) {
                if tile == Tile::Floor && game_map.room_positions.get(&(nx, ny)).is_none() {
                    // This is a corridor - check if it should be visible based on current visibility rules
                    if Self::is_corridor_visible(game_map, player, nx, ny) {
                        return true;
                    }
                }
            }
        }
        false
    }
    
    /// Check if a corridor tile should be visible (helper function)
    fn is_corridor_visible(game_map: &GameMap, player: &Player, x: i32, y: i32) -> bool {
        // Check if any adjacent explored room makes this corridor visible
        for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x + dx;
            let ny = y + dy;
            if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                if player.explored_rooms.contains(&room_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Initialize player exploration for a new dungeon
    pub fn initialize_dungeon_exploration(game_map: &GameMap, player: &mut Player) {
        // Clear previous exploration data
        player.opened_doors.clear();
        player.explored_rooms.clear();
        
        // Find the starting room (containing the dungeon exit)
        for (pos, &tile) in &game_map.tiles {
            if tile == Tile::DungeonExit {
                if let Some(&room_id) = game_map.room_positions.get(pos) {
                    player.explored_rooms.insert(room_id);
                }
                break;
            }
        }
    }

    /// Check if a corridor is connected to an explored area through opened doors
    fn is_corridor_connected_to_explored_area(game_map: &GameMap, player: &Player, start_x: i32, start_y: i32) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start_x, start_y));
        
        while let Some((x, y)) = queue.pop_front() {
            if visited.contains(&(x, y)) {
                continue;
            }
            visited.insert((x, y));
            
            // Check all adjacent positions
            for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x + dx;
                let ny = y + dy;
                
                if visited.contains(&(nx, ny)) {
                    continue;
                }
                
                // Check if adjacent to an explored room
                if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                    if player.explored_rooms.contains(&room_id) {
                        return true;
                    }
                }
                
                // Check if there's a walkable path
                if let Some(&tile) = game_map.tiles.get(&(nx, ny)) {
                    match tile {
                        Tile::Floor => {
                            // If it's a corridor, continue searching
                            if game_map.room_positions.get(&(nx, ny)).is_none() {
                                queue.push_back((nx, ny));
                            }
                        },
                        Tile::Door => {
                            // If it's an opened door, continue searching through it
                            if player.opened_doors.contains(&(nx, ny)) {
                                queue.push_back((nx, ny));
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        
        false
    }

    /// Initialize player exploration for a new dungeon (NetworkPlayer version)
    pub fn initialize_network_player_dungeon_exploration(game_map: &GameMap, player: &mut super::protocol::NetworkPlayer) {
        // Clear previous exploration data
        player.opened_doors.clear();
        player.explored_rooms.clear();
        
        // Find the starting room (containing the dungeon exit)
        for (pos, &tile) in &game_map.tiles {
            if tile == Tile::DungeonExit {
                if let Some(&room_id) = game_map.room_positions.get(pos) {
                    player.explored_rooms.insert(room_id);
                }
                break;
            }
        }
    }

    /// Check if a tile should be visible based on room exploration (NetworkPlayer version)
    pub fn is_tile_visible_network_player(game_map: &GameMap, player: &super::protocol::NetworkPlayer, x: i32, y: i32) -> bool {
        // In overworld, all tiles are always visible
        if game_map.rooms.is_empty() {
            return true;
        }
        
        // Check if position is in a room
        if let Some(&room_id) = game_map.room_positions.get(&(x, y)) {
            // Tile is visible if the room has been explored
            return player.explored_rooms.contains(&room_id);
        }
        
        // Check if position is a corridor (not in any room)
        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
            if tile == Tile::Floor {
                // For corridors, check if any adjacent explored room makes it visible
                for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                        if player.explored_rooms.contains(&room_id) {
                            return true;
                        }
                    }
                }
                
                // Also check if connected to the corridor network through opened doors
                if Self::is_corridor_connected_to_explored_area_network_player(game_map, player, x, y) {
                    return true;
                }
            }
        }
        
        // Doors are visible if they connect to an explored room
        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
            if tile == Tile::Door {
                // Check if door has been opened
                if player.opened_doors.contains(&(x, y)) {
                    return true;
                }
                
                // Check if door is adjacent to an explored room
                for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                        if player.explored_rooms.contains(&room_id) {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Dungeon exit is always visible
        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
            if tile == Tile::DungeonExit {
                return true;
            }
        }
        
        false
    }

    /// Check if a corridor is connected to an explored area through opened doors (NetworkPlayer version)
    fn is_corridor_connected_to_explored_area_network_player(game_map: &GameMap, player: &super::protocol::NetworkPlayer, start_x: i32, start_y: i32) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start_x, start_y));
        
        while let Some((x, y)) = queue.pop_front() {
            if visited.contains(&(x, y)) {
                continue;
            }
            visited.insert((x, y));
            
            // Check all adjacent positions
            for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x + dx;
                let ny = y + dy;
                
                if visited.contains(&(nx, ny)) {
                    continue;
                }
                
                // Check if adjacent to an explored room
                if let Some(&room_id) = game_map.room_positions.get(&(nx, ny)) {
                    if player.explored_rooms.contains(&room_id) {
                        return true;
                    }
                }
                
                // Check if there's a walkable path
                if let Some(&tile) = game_map.tiles.get(&(nx, ny)) {
                    match tile {
                        Tile::Floor => {
                            // If it's a corridor, continue searching
                            if game_map.room_positions.get(&(nx, ny)).is_none() {
                                queue.push_back((nx, ny));
                            }
                        },
                        Tile::Door => {
                            // If it's an opened door, continue searching through it
                            if player.opened_doors.contains(&(nx, ny)) {
                                queue.push_back((nx, ny));
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        
        false
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
