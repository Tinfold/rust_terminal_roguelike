use std::collections::HashMap;
use super::terrain::{Tile, GameMap, Room};
use super::constants::GameConstants;

pub struct DungeonGenerator;

impl DungeonGenerator {
    /// Generate a dungeon map with default parameters
    pub fn generate_dungeon(width: i32, height: i32) -> GameMap {
        let mut game_map = GameMap {
            width,
            height,
            tiles: HashMap::new(),
            rooms: Vec::new(),
            room_positions: HashMap::new(),
        };
        
        // Use a random seed based on current time for variety
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u32;
        
        // Use a new procedural dungeon generation system with rooms and corridors
        Self::generate_procedural_dungeon(&mut game_map, seed);
        
        game_map
    }

    /// Generate a dungeon map with a specific seed for consistency
    pub fn generate_dungeon_with_seed(width: i32, height: i32, seed: u32) -> GameMap {
        let mut game_map = GameMap {
            width,
            height,
            tiles: HashMap::new(),
            rooms: Vec::new(),
            room_positions: HashMap::new(),
        };
        
        // Use a new procedural dungeon generation system with rooms and corridors
        Self::generate_procedural_dungeon(&mut game_map, seed);
        
        game_map
    }

    /// Generate a dungeon map based on entrance position for uniqueness
    pub fn generate_dungeon_for_entrance(entrance_x: i32, entrance_y: i32) -> GameMap {
        let width = GameConstants::DUNGEON_WIDTH;
        let height = GameConstants::DUNGEON_HEIGHT;
        
        // Generate a unique seed based on entrance position
        let seed = Self::generate_dungeon_seed(entrance_x, entrance_y);
        
        Self::generate_dungeon_with_seed(width, height, seed)
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

    /// Get a safe spawn position in a given dungeon map
    pub fn get_safe_spawn_position(dungeon_map: &GameMap) -> (i32, i32) {
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

    /// Get default dungeon spawn position
    pub fn get_default_spawn_position() -> (i32, i32) {
        (GameConstants::DUNGEON_SPAWN_X, GameConstants::DUNGEON_SPAWN_Y)
    }

    fn generate_procedural_dungeon(game_map: &mut GameMap, seed: u32) {
        // Initialize entire dungeon with walls
        for x in 0..game_map.width {
            for y in 0..game_map.height {
                game_map.tiles.insert((x, y), Tile::Wall);
            }
        }

        // Enhanced room generation parameters for larger dungeons
        let min_room_size = 5;
        let max_room_size = 12;
        let max_rooms = 15; // More rooms for larger dungeons
        let mut rooms = Vec::new();
        let mut rng_seed = seed;
        let mut room_id = 0u32; // Room ID counter for exploration tracking

        // Generate a helper function for pseudo-random numbers
        let mut next_random = || {
            rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
            rng_seed
        };

        // Try to place rooms with more attempts for better space utilization
        let max_attempts = max_rooms * 3;
        for _ in 0..max_attempts {
            if rooms.len() >= max_rooms {
                break;
            }

            let room_width = min_room_size + (next_random() % (max_room_size - min_room_size + 1) as u32) as i32;
            let room_height = min_room_size + (next_random() % (max_room_size - min_room_size + 1) as u32) as i32;
            
            let room_x = 2 + (next_random() % (game_map.width - room_width - 4) as u32) as i32;
            let room_y = 2 + (next_random() % (game_map.height - room_height - 4) as u32) as i32;
            
            let new_room = Room {
                x: room_x,
                y: room_y,
                width: room_width,
                height: room_height,
                id: room_id,
            };
            room_id += 1;

            // Check if room overlaps with existing ones (with padding)
            let mut overlaps = false;
            for existing_room in &rooms {
                if Self::rooms_overlap_with_padding(&new_room, existing_room, 2) {
                    overlaps = true;
                    break;
                }
            }

            if !overlaps {
                // Create the room
                Self::create_room(game_map, &new_room);
                rooms.push(new_room);
            }
        }

        // Ensure we have at least one room for spawning
        if rooms.is_empty() {
            let fallback_room = Room {
                x: 3,
                y: 3,
                width: 8,
                height: 6,
                id: room_id,
            };
            Self::create_room(game_map, &fallback_room);
            rooms.push(fallback_room);
        }

        // Create a more sophisticated corridor system
        Self::create_corridor_network(game_map, &rooms, &mut next_random);

        // Add strategic doors to rooms
        Self::add_strategic_doors(game_map, &rooms, &mut next_random);
        
        // Ensure proper door spacing and coverage
        Self::ensure_door_spacing(game_map, &rooms);

        // Ensure spawn position is on a floor tile
        Self::ensure_safe_spawn_position(game_map, &rooms);
        
        // Store room information in the GameMap for visibility tracking
        game_map.rooms = rooms.clone();
        Self::populate_room_positions(game_map);
    }

    fn rooms_overlap_with_padding(room1: &Room, room2: &Room, padding: i32) -> bool {
        room1.x - padding < room2.x + room2.width &&
        room1.x + room1.width + padding > room2.x &&
        room1.y - padding < room2.y + room2.height &&
        room1.y + room1.height + padding > room2.y
    }

    fn create_room(game_map: &mut GameMap, room: &Room) {
        for x in room.x..room.x + room.width {
            for y in room.y..room.y + room.height {
                if x > 0 && x < game_map.width - 1 && y > 0 && y < game_map.height - 1 {
                    game_map.tiles.insert((x, y), Tile::Floor);
                }
            }
        }
    }

    fn room_center(room: &Room) -> (i32, i32) {
        (room.x + room.width / 2, room.y + room.height / 2)
    }

    fn create_corridor(game_map: &mut GameMap, start: (i32, i32), end: (i32, i32)) {
        let (mut x, mut y) = start;
        let (target_x, target_y) = end;

        // Create L-shaped corridor
        // First move horizontally
        while x != target_x {
            if x > 0 && x < game_map.width - 1 && y > 0 && y < game_map.height - 1 {
                game_map.tiles.insert((x, y), Tile::Floor);
            }
            x += if target_x > x { 1 } else { -1 };
        }

        // Then move vertically
        while y != target_y {
            if x > 0 && x < game_map.width - 1 && y > 0 && y < game_map.height - 1 {
                game_map.tiles.insert((x, y), Tile::Floor);
            }
            y += if target_y > y { 1 } else { -1 };
        }

        // Ensure the endpoint is also a floor
        if x > 0 && x < game_map.width - 1 && y > 0 && y < game_map.height - 1 {
            game_map.tiles.insert((x, y), Tile::Floor);
        }
    }

    fn create_corridor_network(game_map: &mut GameMap, rooms: &[Room], next_random: &mut impl FnMut() -> u32) {
        if rooms.len() < 2 {
            return;
        }

        // Create a minimum spanning tree of connections
        let mut connected = vec![false; rooms.len()];
        connected[0] = true;
        let mut connections_made = 1;

        while connections_made < rooms.len() {
            let mut best_distance = f32::MAX;
            let mut best_connected = 0;
            let mut best_unconnected = 0;

            // Find the shortest connection between a connected and unconnected room
            for i in 0..rooms.len() {
                if !connected[i] {
                    continue;
                }
                for j in 0..rooms.len() {
                    if connected[j] {
                        continue;
                    }
                    let center_i = Self::room_center(&rooms[i]);
                    let center_j = Self::room_center(&rooms[j]);
                    let distance = ((center_i.0 - center_j.0).pow(2) + (center_i.1 - center_j.1).pow(2)) as f32;
                    
                    if distance < best_distance {
                        best_distance = distance;
                        best_connected = i;
                        best_unconnected = j;
                    }
                }
            }

            // Connect the best pair
            let start = Self::room_center(&rooms[best_connected]);
            let end = Self::room_center(&rooms[best_unconnected]);
            Self::create_corridor(game_map, start, end);
            connected[best_unconnected] = true;
            connections_made += 1;
        }

        // Add some extra connections for more interesting layouts (25% chance per room pair)
        for i in 0..rooms.len() {
            for j in i+1..rooms.len() {
                if next_random() % 4 == 0 { // 25% chance
                    let start = Self::room_center(&rooms[i]);
                    let end = Self::room_center(&rooms[j]);
                    Self::create_corridor(game_map, start, end);
                }
            }
        }
    }

    fn add_strategic_doors(game_map: &mut GameMap, rooms: &[Room], next_random: &mut impl FnMut() -> u32) {
        for room in rooms {
            // Find all potential door positions (walls that should become doors)
            let mut door_candidates = Vec::new();
            
            // Check all perimeter positions of the room
            for x in room.x-1..=room.x+room.width {
                for y in room.y-1..=room.y+room.height {
                    // Skip corners and positions outside bounds
                    if x <= 0 || x >= game_map.width - 1 || 
                       y <= 0 || y >= game_map.height - 1 {
                        continue;
                    }
                    
                    // Check if this is a wall tile
                    if game_map.tiles.get(&(x, y)) != Some(&Tile::Wall) {
                        continue;
                    }
                    
                    // Check if it's on the room perimeter
                    let on_perimeter = (x == room.x - 1 || x == room.x + room.width) && 
                                      (y >= room.y && y < room.y + room.height) ||
                                      (y == room.y - 1 || y == room.y + room.height) && 
                                      (x >= room.x && x < room.x + room.width);
                    
                    if !on_perimeter {
                        continue;
                    }
                    
                    // Check if there's a floor tile on both sides (indicating a corridor connection)
                    let has_floor_inside = Self::has_adjacent_floor_in_room(game_map, x, y, room);
                    let has_floor_outside = Self::has_adjacent_floor_outside_room(game_map, x, y, room);
                    
                    // Only place doors where there's a clear passage (floor on both sides)
                    // and ensure the door is properly encased
                    if has_floor_inside && has_floor_outside && Self::is_door_properly_encased(game_map, x, y, room) {
                        door_candidates.push((x, y));
                    }
                }
            }
            
            // Place doors at strategic locations with better distribution
            for &(door_x, door_y) in &door_candidates {
                // Higher chance for doors on rooms with fewer existing doors
                let current_doors = Self::count_doors_in_room(game_map, room);
                let door_chance = if current_doors == 0 {
                    80 // 80% chance for first door
                } else if current_doors == 1 {
                    40 // 40% chance for second door
                } else {
                    15 // 15% chance for additional doors
                };
                
                if (next_random() % 100) < door_chance as u32 {
                    game_map.tiles.insert((door_x, door_y), Tile::Door);
                }
            }
            
            // Ensure every room has at least one door
            if Self::count_doors_in_room(game_map, room) == 0 && !door_candidates.is_empty() {
                let door_idx = (next_random() % door_candidates.len() as u32) as usize;
                let (door_x, door_y) = door_candidates[door_idx];
                game_map.tiles.insert((door_x, door_y), Tile::Door);
            }
        }
    }
    
    fn has_adjacent_floor_in_room(game_map: &GameMap, x: i32, y: i32, room: &Room) -> bool {
        for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x + dx;
            let ny = y + dy;
            
            // Check if this position is inside the room
            if nx >= room.x && nx < room.x + room.width && 
               ny >= room.y && ny < room.y + room.height {
                if game_map.tiles.get(&(nx, ny)) == Some(&Tile::Floor) {
                    return true;
                }
            }
        }
        false
    }
    
    fn has_adjacent_floor_outside_room(game_map: &GameMap, x: i32, y: i32, room: &Room) -> bool {
        for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x + dx;
            let ny = y + dy;
            
            // Check if this position is outside the room
            if nx < room.x || nx >= room.x + room.width || 
               ny < room.y || ny >= room.y + room.height {
                if game_map.tiles.get(&(nx, ny)) == Some(&Tile::Floor) {
                    return true;
                }
            }
        }
        false
    }
    
    fn count_doors_in_room(game_map: &GameMap, room: &Room) -> i32 {
        let mut count = 0;
        
        // Check perimeter of room for doors
        for x in room.x-1..=room.x+room.width {
            for y in room.y-1..=room.y+room.height {
                // Check if it's on the room perimeter
                let on_perimeter = (x == room.x - 1 || x == room.x + room.width) && 
                                  (y >= room.y && y < room.y + room.height) ||
                                  (y == room.y - 1 || y == room.y + room.height) && 
                                  (x >= room.x && x < room.x + room.width);
                
                if on_perimeter && game_map.tiles.get(&(x, y)) == Some(&Tile::Door) {
                    count += 1;
                }
            }
        }
        
        count
    }

    fn ensure_safe_spawn_position(game_map: &mut GameMap, rooms: &[Room]) {
        if rooms.is_empty() {
            return;
        }

        // Use the center of the first room as spawn point
        let spawn_room = &rooms[0];
        let (spawn_x, spawn_y) = Self::room_center(spawn_room);
        
        // Place a dungeon exit at the spawn position - this represents the entrance from the overworld
        game_map.tiles.insert((spawn_x, spawn_y), Tile::DungeonExit);
        
        // Create a larger clearing around the exit for better accessibility
        // Clear a 3x3 area around the exit
        for dx in -1..=1 {
            for dy in -1..=1 {
                let x = spawn_x + dx;
                let y = spawn_y + dy;
                if x > 0 && x < game_map.width - 1 && y > 0 && y < game_map.height - 1 {
                    // Only convert walls to floors, leave existing floors and doors alone
                    // Don't overwrite the dungeon exit tile we just placed
                    if game_map.tiles.get(&(x, y)) == Some(&Tile::Wall) {
                        game_map.tiles.insert((x, y), Tile::Floor);
                    }
                }
            }
        }
        
        // Also ensure a slightly larger 5x5 area has traversable space, but only convert walls
        for dx in -2..=2 {
            for dy in -2..=2 {
                let x = spawn_x + dx;
                let y = spawn_y + dy;
                if x > 0 && x < game_map.width - 1 && y > 0 && y < game_map.height - 1 {
                    // Only create floor tiles where there are walls, creating a more natural clearing
                    if game_map.tiles.get(&(x, y)) == Some(&Tile::Wall) && 
                       (dx.abs() <= 1 || dy.abs() <= 1) { // Ensure at least the cross pattern is clear
                        game_map.tiles.insert((x, y), Tile::Floor);
                    }
                }
            }
        }
    }

    /// Populate the room_positions map with position -> room_id mapping
    fn populate_room_positions(game_map: &mut GameMap) {
        game_map.room_positions.clear();
        for room in &game_map.rooms {
            for x in room.x..room.x + room.width {
                for y in room.y..room.y + room.height {
                    game_map.room_positions.insert((x, y), room.id);
                }
            }
        }
    }

    /// Ensure doors are properly spaced and not clustered together
    fn ensure_door_spacing(game_map: &mut GameMap, rooms: &[Room]) {
        let mut doors_to_remove = Vec::new();
        let mut all_doors = Vec::new();
        
        // Collect all door positions
        for (pos, tile) in &game_map.tiles {
            if *tile == Tile::Door {
                all_doors.push(*pos);
            }
        }
        
        // Check for doors that are too close to each other
        for i in 0..all_doors.len() {
            for j in i+1..all_doors.len() {
                let (x1, y1) = all_doors[i];
                let (x2, y2) = all_doors[j];
                let distance = ((x2 - x1).abs() + (y2 - y1).abs()) as f32;
                
                // If doors are too close (less than 3 tiles apart), remove one
                if distance < 3.0 {
                    // Keep the door that's more strategically placed
                    let door1_connections = Self::count_door_connections(game_map, x1, y1);
                    let door2_connections = Self::count_door_connections(game_map, x2, y2);
                    
                    if door1_connections < door2_connections {
                        doors_to_remove.push((x1, y1));
                    } else {
                        doors_to_remove.push((x2, y2));
                    }
                }
            }
        }
        
        // Remove excessive doors
        for door_pos in doors_to_remove {
            game_map.tiles.insert(door_pos, Tile::Wall);
        }
        
        // Ensure each room has at least one door
        for room in rooms {
            if Self::count_doors_in_room(game_map, room) == 0 {
                // Find a good spot for a door
                if let Some(door_pos) = Self::find_best_door_position(game_map, room) {
                    game_map.tiles.insert(door_pos, Tile::Door);
                }
            }
        }
    }
    
    /// Count how many floor tiles are adjacent to a door position
    fn count_door_connections(game_map: &GameMap, door_x: i32, door_y: i32) -> i32 {
        let mut connections = 0;
        for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = door_x + dx;
            let ny = door_y + dy;
            if game_map.tiles.get(&(nx, ny)) == Some(&Tile::Floor) {
                connections += 1;
            }
        }
        connections
    }
    
    /// Find the best position for a door in a room
    fn find_best_door_position(game_map: &GameMap, room: &Room) -> Option<(i32, i32)> {
        let mut best_pos = None;
        let mut best_connections = 0;
        
        // Check all perimeter positions
        for x in room.x-1..=room.x+room.width {
            for y in room.y-1..=room.y+room.height {
                // Skip corners and out of bounds
                if x <= 0 || x >= game_map.width - 1 || 
                   y <= 0 || y >= game_map.height - 1 {
                    continue;
                }
                
                // Check if this is a wall on the room perimeter
                if game_map.tiles.get(&(x, y)) != Some(&Tile::Wall) {
                    continue;
                }
                
                let on_perimeter = (x == room.x - 1 || x == room.x + room.width) && 
                                  (y >= room.y && y < room.y + room.height) ||
                                  (y == room.y - 1 || y == room.y + room.height) && 
                                  (x >= room.x && x < room.x + room.width);
                
                if !on_perimeter {
                    continue;
                }
                
                // Check connections
                let has_floor_inside = Self::has_adjacent_floor_in_room(game_map, x, y, room);
                let has_floor_outside = Self::has_adjacent_floor_outside_room(game_map, x, y, room);
                
                if has_floor_inside && has_floor_outside {
                    let connections = Self::count_door_connections(game_map, x, y);
                    if connections > best_connections {
                        best_connections = connections;
                        best_pos = Some((x, y));
                    }
                }
            }
        }
        
        best_pos
    }

    /// Check if a door position is properly encased (surrounded by walls except for the passage)
    fn is_door_properly_encased(game_map: &GameMap, door_x: i32, door_y: i32, room: &Room) -> bool {
        // Determine if the door is on a horizontal or vertical wall
        let is_horizontal_wall = door_y == room.y - 1 || door_y == room.y + room.height;
        
        if is_horizontal_wall {
            // For horizontal walls, check that walls exist on left and right sides
            let left_wall = game_map.tiles.get(&(door_x - 1, door_y)) == Some(&Tile::Wall);
            let right_wall = game_map.tiles.get(&(door_x + 1, door_y)) == Some(&Tile::Wall);
            
            // Ensure there's a clear passage above and below
            let passage_exists = (door_y > 0 && door_y < game_map.height - 1) &&
                                ((game_map.tiles.get(&(door_x, door_y - 1)) == Some(&Tile::Floor)) ||
                                 (game_map.tiles.get(&(door_x, door_y + 1)) == Some(&Tile::Floor)));
            
            left_wall && right_wall && passage_exists
        } else {
            // For vertical walls, check that walls exist above and below
            let top_wall = game_map.tiles.get(&(door_x, door_y - 1)) == Some(&Tile::Wall);
            let bottom_wall = game_map.tiles.get(&(door_x, door_y + 1)) == Some(&Tile::Wall);
            
            // Ensure there's a clear passage to left and right
            let passage_exists = (door_x > 0 && door_x < game_map.width - 1) &&
                                ((game_map.tiles.get(&(door_x - 1, door_y)) == Some(&Tile::Floor)) ||
                                 (game_map.tiles.get(&(door_x + 1, door_y)) == Some(&Tile::Floor)));
            
            top_wall && bottom_wall && passage_exists
        }
    }
}
