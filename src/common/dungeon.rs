use crate::common::terrain::{GameMap, Tile, Room, RoomType};
use std::collections::HashSet;

/// BSP-based dungeon generator with player lighting system
pub struct DungeonGenerator;

/// BSP Node for recursive dungeon generation
#[derive(Debug, Clone)]
struct BSPNode {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    left: Option<Box<BSPNode>>,
    right: Option<Box<BSPNode>>,
    room: Option<Room>,
    id: u32,
}

impl BSPNode {
    fn new(x: i32, y: i32, width: i32, height: i32, id: u32) -> Self {
        BSPNode {
            x,
            y,
            width,
            height,
            left: None,
            right: None,
            room: None,
            id,
        }
    }

    /// Check if this node can be split
    fn can_split(&self, min_size: i32) -> bool {
        println!("Checking if node {}x{} can split with min_size {}", self.width, self.height, min_size);
        // Use AND instead of OR to ensure both dimensions are reasonable
        let can_split = self.width > min_size * 2 && self.height > min_size * 2;
        println!("  Result: {}", can_split);
        can_split
    }

    /// Split this node into two child nodes
    fn split(&mut self, next_id: &mut u32, min_size: i32) -> bool {
        if !self.can_split(min_size) {
            return false;
        }

        // Add more randomness to split direction decision
        let split_horizontal = if (self.width as f32) >= 1.25 * (self.height as f32) {
            false // Split vertically if significantly wider
        } else if (self.height as f32) >= 1.25 * (self.width as f32) {
            true // Split horizontally if significantly taller
        } else {
            // Random choice with better distribution
            (self.id + *next_id) % 2 == 0
        };

        let (max_split, min_split_size) = if split_horizontal {
            (self.height - min_size, min_size)
        } else {
            (self.width - min_size, min_size)
        };

        if max_split <= min_split_size {
            return false;
        }

        // Better distribution for split position - avoid middle splits too often
        let split_pos = min_split_size + 
            ((self.id.wrapping_mul(17) + *next_id * 13) % (max_split - min_split_size) as u32) as i32;

        if split_horizontal {
            // Horizontal split - create top and bottom children
            self.left = Some(Box::new(BSPNode::new(
                self.x,
                self.y,
                self.width,
                split_pos,
                *next_id,
            )));
            *next_id += 1;
            
            self.right = Some(Box::new(BSPNode::new(
                self.x,
                self.y + split_pos,
                self.width,
                self.height - split_pos,
                *next_id,
            )));
            *next_id += 1;
        } else {
            // Vertical split - create left and right children
            self.left = Some(Box::new(BSPNode::new(
                self.x,
                self.y,
                split_pos,
                self.height,
                *next_id,
            )));
            *next_id += 1;
            
            self.right = Some(Box::new(BSPNode::new(
                self.x + split_pos,
                self.y,
                self.width - split_pos,
                self.height,
                *next_id,
            )));
            *next_id += 1;
        }

        true
    }

    /// Create rooms in leaf nodes
    fn create_rooms(&mut self, min_room_size: i32, max_room_size: i32) {
        if let (Some(ref mut left), Some(ref mut right)) = (&mut self.left, &mut self.right) {
            // This is an internal node - recurse to children
            left.create_rooms(min_room_size, max_room_size);
            right.create_rooms(min_room_size, max_room_size);
        } else {
            // This is a leaf node - create a room
            let margin = 2; // Leave some space from the edges
            let max_width = (self.width - margin * 2).min(max_room_size);
            let max_height = (self.height - margin * 2).min(max_room_size);
            
            if max_width >= min_room_size && max_height >= min_room_size {
                let room_width = min_room_size + (self.id * 13) as i32 % (max_width - min_room_size + 1);
                let room_height = min_room_size + (self.id * 19) as i32 % (max_height - min_room_size + 1);
                
                let room_x = self.x + margin + (self.id * 23) as i32 % (self.width - room_width - margin * 2 + 1);
                let room_y = self.y + margin + (self.id * 29) as i32 % (self.height - room_height - margin * 2 + 1);
                
                self.room = Some(Room {
                    x: room_x,
                    y: room_y,
                    width: room_width,
                    height: room_height,
                    id: self.id,
                    room_type: RoomType::Rectangle,
                    is_illuminated: false,
                    connected_rooms: Vec::new(),
                });
            }
        }
    }

    /// Get all rooms from this node and its children
    fn get_rooms(&self, rooms: &mut Vec<Room>) {
        if let Some(ref room) = self.room {
            rooms.push(room.clone());
        }
        
        if let Some(ref left) = self.left {
            left.get_rooms(rooms);
        }
        
        if let Some(ref right) = self.right {
            right.get_rooms(rooms);
        }
    }

    /// Get center point of this node's room (if it has one)
    fn get_room_center(&self) -> Option<(i32, i32)> {
        if let Some(ref room) = self.room {
            Some((room.x + room.width / 2, room.y + room.height / 2))
        } else {
            None
        }
    }

    /// Get the center point for connecting to other nodes
    fn get_connection_center(&self) -> (i32, i32) {
        if let Some(center) = self.get_room_center() {
            center
        } else {
            // For internal nodes, find the center between child connection points
            match (&self.left, &self.right) {
                (Some(left), Some(right)) => {
                    let left_center = left.get_connection_center();
                    let right_center = right.get_connection_center();
                    ((left_center.0 + right_center.0) / 2, (left_center.1 + right_center.1) / 2)
                },
                _ => (self.x + self.width / 2, self.y + self.height / 2)
            }
        }
    }

    /// Connect this node's children with corridors
    fn connect_children(&self, game_map: &mut GameMap) {
        if let (Some(ref left), Some(ref right)) = (&self.left, &self.right) {
            // First, recursively connect children
            left.connect_children(game_map);
            right.connect_children(game_map);
            
            // Then connect the two subtrees
            let left_center = left.get_connection_center();
            let right_center = right.get_connection_center();
            
            DungeonGenerator::carve_l_shaped_corridor(game_map, left_center, right_center);
        }
    }
}

impl DungeonGenerator {
    /// Generate a BSP-based dungeon with rooms and corridors
    pub fn generate_dungeon(width: i32, height: i32) -> GameMap {
        let mut game_map = GameMap::new(width, height);
        
        // Fill with walls initially
        for x in 0..width {
            for y in 0..height {
                game_map.tiles.insert((x, y), Tile::Wall);
            }
        }
        
        // Create BSP tree
        let mut root = BSPNode::new(1, 1, width - 2, height - 2, 0);
        let mut next_id = 1;
        
        println!("Starting BSP generation with root: {}x{} at ({}, {})", root.width, root.height, root.x, root.y);
        
        // Split the space recursively with parameters tuned for dungeon size
        let min_size = if width >= 80 && height >= 40 {
            12  // Larger dungeons can have bigger minimum partition sizes
        } else {
            10  // Increased significantly to fix room creation
        };
        let max_depth = if width >= 80 && height >= 40 {
            5  // More depth for larger dungeons
        } else {
            3  // Reduced to prevent over-splitting small spaces
        };
        
        Self::split_node_recursive(&mut root, &mut next_id, min_size, max_depth);
        
        // Debug the BSP tree structure - Add this line!
        println!("BSP tree structure:");
        debug_bsp_tree(&root, 0);
        
        // Create rooms in leaf nodes - adjusted parameters
        root.create_rooms(5, 8); // Adjusted room sizes for better fit
        
        // Get all rooms
        let mut rooms = Vec::new();
        root.get_rooms(&mut rooms);
        
        // Debug: Print room count
        println!("BSP Dungeon Generator: Created {} rooms", rooms.len());
        for (i, room) in rooms.iter().enumerate() {
            println!("  Room {}: ({}, {}) {}x{}", i, room.x, room.y, room.width, room.height);
        }
        
        // Carve out rooms
        for room in &rooms {
            Self::carve_room(&mut game_map, room);
        }
        
        // Connect rooms with corridors using BSP structure
        root.connect_children(&mut game_map);
        
        // Add doors at corridor-room intersections
        Self::add_doors(&mut game_map, &rooms);
        
        // Add entrance and exit
        if let Some(first_room) = rooms.first() {
            game_map.tiles.insert((first_room.x + 1, first_room.y + 1), Tile::DungeonExit);
        }
        
        // Update room connections based on actual layout
        let mut connected_rooms = rooms;
        Self::update_room_connections(&mut connected_rooms, &game_map);
        
        game_map.rooms = connected_rooms;
        game_map
    }

    /// Recursively split BSP nodes
    fn split_node_recursive(node: &mut BSPNode, next_id: &mut u32, min_size: i32, max_depth: i32) {
        if max_depth <= 0 || !node.can_split(min_size) {
            return;
        }
        
        if node.split(next_id, min_size) {
            if let Some(ref mut left) = node.left {
                Self::split_node_recursive(left, next_id, min_size, max_depth - 1);
            }
            if let Some(ref mut right) = node.right {
                Self::split_node_recursive(right, next_id, min_size, max_depth - 1);
            }
        }
    }
    
    /// Carve out a rectangular room
    fn carve_room(game_map: &mut GameMap, room: &Room) {
        for x in room.x..(room.x + room.width) {
            for y in room.y..(room.y + room.height) {
                if x > 0 && y > 0 && x < game_map.width - 1 && y < game_map.height - 1 {
                    game_map.tiles.insert((x, y), Tile::Floor);
                    game_map.room_positions.insert((x, y), room.id);
                }
            }
        }
    }
    
    /// Carve an L-shaped corridor between two points
    fn carve_l_shaped_corridor(game_map: &mut GameMap, from: (i32, i32), to: (i32, i32)) {
        let (x1, y1) = from;
        let (x2, y2) = to;
        
        // Choose corner point - sometimes go horizontal first, sometimes vertical first
        let corner_horizontal_first = (x1 + y1 + x2 + y2) % 2 == 0;
        
        if corner_horizontal_first {
            // Go horizontal first, then vertical
            Self::carve_corridor_line(game_map, x1, y1, x2, y1);
            Self::carve_corridor_line(game_map, x2, y1, x2, y2);
        } else {
            // Go vertical first, then horizontal
            Self::carve_corridor_line(game_map, x1, y1, x1, y2);
            Self::carve_corridor_line(game_map, x1, y2, x2, y2);
        }
    }
    
    /// Carve a straight corridor line between two points
    fn carve_corridor_line(game_map: &mut GameMap, x1: i32, y1: i32, x2: i32, y2: i32) {
        let mut x = x1;
        let mut y = y1;
        
        while x != x2 || y != y2 {
            // Only carve if it's not already a room floor
            if x > 0 && y > 0 && x < game_map.width - 1 && y < game_map.height - 1 {
                if let Some(&tile) = game_map.tiles.get(&(x, y)) {
                    if tile == Tile::Wall {
                        game_map.tiles.insert((x, y), Tile::Corridor);
                    }
                }
            }
            
            // Move towards the target
            if x < x2 {
                x += 1;
            } else if x > x2 {
                x -= 1;
            } else if y < y2 {
                y += 1;
            } else if y > y2 {
                y -= 1;
            }
        }
    }
    
    /// Add doors at room-corridor intersections
    fn add_doors(game_map: &mut GameMap, rooms: &[Room]) {
        let mut door_positions = HashSet::new();
        
        // Find all positions where corridors meet rooms
        for room in rooms {
            // Check the perimeter of each room
            for x in (room.x - 1)..=(room.x + room.width) {
                for y in (room.y - 1)..=(room.y + room.height) {
                    // Check if this position is on the room's border
                    let on_border = (x == room.x - 1 || x == room.x + room.width ||
                                     y == room.y - 1 || y == room.y + room.height) &&
                                    x >= room.x - 1 && x <= room.x + room.width &&
                                    y >= room.y - 1 && y <= room.y + room.height;
                    
                    if on_border {
                        // Check if there's a corridor adjacent to the room
                        if let Some(&tile) = game_map.tiles.get(&(x, y)) {
                            if tile == Tile::Corridor {
                                // Check if there's a room floor adjacent
                                let adjacent_positions = [
                                    (x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)
                                ];
                                
                                for &(ax, ay) in &adjacent_positions {
                                    if let Some(&adj_tile) = game_map.tiles.get(&(ax, ay)) {
                                        if adj_tile == Tile::Floor {
                                            if let Some(&room_id) = game_map.room_positions.get(&(ax, ay)) {
                                                if room_id == room.id {
                                                    // This corridor connects to this room - place a door
                                                    door_positions.insert((x, y));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Place the doors
        for &(x, y) in &door_positions {
            game_map.tiles.insert((x, y), Tile::Door);
        }
    }
    
    /// Update room connections based on door placement
    fn update_room_connections(rooms: &mut Vec<Room>, game_map: &GameMap) {
        // Clear existing connections
        for room in rooms.iter_mut() {
            room.connected_rooms.clear();
        }
        
        // Find connections through doors
        for &(door_x, door_y) in game_map.tiles.iter().filter_map(|(pos, tile)| {
            if *tile == Tile::Door { Some(pos) } else { None }
        }) {
            let mut connected_room_ids = Vec::new();
            
            // Check adjacent positions for rooms
            for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let adj_x = door_x + dx;
                let adj_y = door_y + dy;
                
                if let Some(&room_id) = game_map.room_positions.get(&(adj_x, adj_y)) {
                    if !connected_room_ids.contains(&room_id) {
                        connected_room_ids.push(room_id);
                    }
                }
            }
            
            // Connect the rooms bidirectionally
            for &room_id_1 in &connected_room_ids {
                for &room_id_2 in &connected_room_ids {
                    if room_id_1 != room_id_2 {
                        // Find both rooms and ensure bidirectional connection
                        if let Some(room_1) = rooms.iter_mut().find(|r| r.id == room_id_1) {
                            if !room_1.connected_rooms.contains(&room_id_2) {
                                room_1.connected_rooms.push(room_id_2);
                            }
                        }
                        
                        if let Some(room_2) = rooms.iter_mut().find(|r| r.id == room_id_2) {
                            if !room_2.connected_rooms.contains(&room_id_1) {
                                room_2.connected_rooms.push(room_id_1);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Generate a dungeon map based on entrance position for uniqueness
    pub fn generate_dungeon_for_entrance(entrance_x: i32, entrance_y: i32) -> GameMap {
        use crate::common::constants::GameConstants;
        // Use entrance position as seed for consistent generation
        let seed = Self::generate_dungeon_seed(entrance_x, entrance_y);
        Self::generate_dungeon_with_seed(GameConstants::DUNGEON_WIDTH, GameConstants::DUNGEON_HEIGHT, seed)
    }
    
    /// Generate a dungeon seed based on entrance position
    pub fn generate_dungeon_seed(entrance_x: i32, entrance_y: i32) -> u32 {
        // Create a deterministic seed from entrance coordinates using wrapping arithmetic
        let x_part = (entrance_x as u32).wrapping_mul(31);
        let y_part = (entrance_y as u32).wrapping_mul(17);
        x_part.wrapping_add(y_part) ^ 0x12345678
    }
    
    /// Generate a dungeon map with a specific seed for consistency
    pub fn generate_dungeon_with_seed(width: i32, height: i32, _seed: u32) -> GameMap {
        // For now, just use the basic generation (can be enhanced later with seed-based randomization)
        Self::generate_dungeon(width, height)
    }
    
    /// Get default dungeon spawn position
    pub fn get_default_spawn_position() -> (i32, i32) {
        (6, 8) // Inside the first room
    }
    
    /// Get a safe spawn position in a given dungeon map
    pub fn get_safe_spawn_position(dungeon_map: &GameMap) -> (i32, i32) {
        // Try to find a floor tile, starting from the default position
        let default_pos = Self::get_default_spawn_position();
        
        // Check if default position is valid
        if let Some(tile) = dungeon_map.tiles.get(&default_pos) {
            if *tile == Tile::Floor || *tile == Tile::DungeonExit {
                return default_pos;
            }
        }
        
        // Find any floor tile
        for x in 1..dungeon_map.width {
            for y in 1..dungeon_map.height {
                if let Some(tile) = dungeon_map.tiles.get(&(x, y)) {
                    if *tile == Tile::Floor || *tile == Tile::DungeonExit {
                        return (x, y);
                    }
                }
            }
        }
        
        // Fallback to default position even if not ideal
        default_pos
    }

    /// Save dungeon visualization to a file
    pub fn save_visualization(dungeon: &GameMap, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        use crate::visualization::DungeonVisualizer;
        DungeonVisualizer::save_dungeon_bitmap(dungeon, filename)
    }
}

/// Player lighting system with distance-based brightness
#[derive(Debug, Clone)]
pub struct LightLevel {
    pub brightness: f32, // 0.0 to 1.0, where 1.0 is fully lit
}

impl LightLevel {
    pub fn new(brightness: f32) -> Self {
        Self {
            brightness: brightness.clamp(0.0, 1.0),
        }
    }
    
    pub fn dark() -> Self {
        Self { brightness: 0.0 }
    }
    
    pub fn bright() -> Self {
        Self { brightness: 1.0 }
    }
}

/// Enhanced GameMap with lighting
impl GameMap {
    /// Update player light and visibility with door awareness
    pub fn update_lighting_with_doors(&mut self, player_x: i32, player_y: i32, light_radius: i32, opened_doors: &std::collections::HashSet<(i32, i32)>) {
        // Clear current visibility and lighting
        self.visible_tiles.clear();
        
        // Calculate lighting for each tile within radius
        for dx in -light_radius..=light_radius {
            for dy in -light_radius..=light_radius {
                let x = player_x + dx;
                let y = player_y + dy;
                
                // Skip if outside map bounds
                if x < 0 || y < 0 || x >= self.width || y >= self.height {
                    continue;
                }
                
                // Calculate distance from player (using safe arithmetic to avoid overflow)
                let dx_f = dx as f32;
                let dy_f = dy as f32;
                let distance = (dx_f * dx_f + dy_f * dy_f).sqrt();
                
                // Only light tiles within radius
                if distance <= light_radius as f32 {
                    // Check line of sight with door awareness
                    if self.has_line_of_sight_with_doors(player_x, player_y, x, y, opened_doors) {
                        // Calculate brightness based on distance
                        let brightness = Self::calculate_brightness(distance, light_radius as f32);
                        
                        // Mark as visible if bright enough
                        if brightness > 0.1 {
                            self.visible_tiles.insert((x, y), true);
                            self.explored_tiles.insert((x, y), true);
                        }
                    }
                }
            }
        }
    }

    /// Update player light and visibility (legacy method - uses door-aware version)
    pub fn update_lighting(&mut self, player_x: i32, player_y: i32, light_radius: i32) {
        // Use empty set for opened doors - this maintains backwards compatibility
        // but doors will block light by default
        let empty_doors = std::collections::HashSet::new();
        self.update_lighting_with_doors(player_x, player_y, light_radius, &empty_doors);
    }
    
    /// Calculate brightness based on distance from light source
    fn calculate_brightness(distance: f32, max_radius: f32) -> f32 {
        if distance <= 1.0 {
            1.0 // Full brightness at player position and adjacent tiles
        } else if distance >= max_radius {
            0.0 // No light beyond max radius
        } else {
            // Linear falloff - you can make this more sophisticated
            let falloff = 1.0 - (distance / max_radius);
            falloff.powi(2) // Quadratic falloff for more realistic lighting
        }
    }
    
    /// Get the light level at a specific position with door awareness
    pub fn get_light_level_with_doors(&self, player_x: i32, player_y: i32, x: i32, y: i32, light_radius: i32, opened_doors: &std::collections::HashSet<(i32, i32)>) -> LightLevel {
        let dx = (x - player_x) as f32;
        let dy = (y - player_y) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        
        if distance <= light_radius as f32 && self.has_line_of_sight_with_doors(player_x, player_y, x, y, opened_doors) {
            let brightness = Self::calculate_brightness(distance, light_radius as f32);
            LightLevel::new(brightness)
        } else {
            LightLevel::dark()
        }
    }

    /// Get the light level at a specific position (legacy method)
    pub fn get_light_level(&self, player_x: i32, player_y: i32, x: i32, y: i32, light_radius: i32) -> LightLevel {
        let empty_doors = std::collections::HashSet::new();
        self.get_light_level_with_doors(player_x, player_y, x, y, light_radius, &empty_doors)
    }
    
    /// Check if a tile should be rendered (visible or explored)
    pub fn should_render_tile(&self, x: i32, y: i32) -> bool {
        self.is_visible(x, y) || self.is_explored(x, y)
    }
    
    /// Get rendering style based on visibility and light level with door awareness
    pub fn get_tile_visibility_state_with_doors(&self, player_x: i32, player_y: i32, x: i32, y: i32, light_radius: i32, opened_doors: &std::collections::HashSet<(i32, i32)>) -> TileVisibility {
        if self.is_visible(x, y) {
            let light_level = self.get_light_level_with_doors(player_x, player_y, x, y, light_radius, opened_doors);
            TileVisibility::Lit(light_level)
        } else if self.is_explored(x, y) {
            TileVisibility::Remembered
        } else {
            TileVisibility::Hidden
        }
    }

    /// Get rendering style based on visibility and light level (legacy method)
    pub fn get_tile_visibility_state(&self, player_x: i32, player_y: i32, x: i32, y: i32, light_radius: i32) -> TileVisibility {
        let empty_doors = std::collections::HashSet::new();
        self.get_tile_visibility_state_with_doors(player_x, player_y, x, y, light_radius, &empty_doors)
    }
}

/// Tile visibility states for rendering
#[derive(Debug, Clone)]
pub enum TileVisibility {
    Hidden,                    // Never seen, don't render
    Remembered,               // Previously seen but not currently visible, render dimly
    Lit(LightLevel),          // Currently visible and lit, render with brightness
}

impl TileVisibility {
    /// Get the brightness factor for rendering (0.0 to 1.0)
    pub fn get_brightness(&self) -> f32 {
        match self {
            TileVisibility::Hidden => 0.0,
            TileVisibility::Remembered => 0.3, // Dim but visible
            TileVisibility::Lit(light) => 0.5 + (light.brightness * 0.5), // 0.5 to 1.0 range
        }
    }
    
    /// Check if tile should be rendered
    pub fn is_visible(&self) -> bool {
        !matches!(self, TileVisibility::Hidden)
    }
}

/// Debug print BSP tree structure
fn debug_bsp_tree(node: &BSPNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}Node {} at ({}, {}) {}x{}", indent, node.id, node.x, node.y, node.width, node.height);
    
    if let Some(ref room) = node.room {
        println!("{}  Room: ({}, {}) {}x{}", indent, room.x, room.y, room.width, room.height);
    }
    
    if let Some(ref left) = node.left {
        println!("{}  Left child:", indent);
        debug_bsp_tree(left, depth + 1);
    }
    
    if let Some(ref right) = node.right {
        println!("{}  Right child:", indent);
        debug_bsp_tree(right, depth + 1);
    }
}