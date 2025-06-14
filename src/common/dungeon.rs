use crate::common::terrain::{GameMap, Tile, Room, RoomType};

/// Simple dungeon generator with player lighting system
pub struct DungeonGenerator;

impl DungeonGenerator {
    /// Generate a basic dungeon with simple rooms and corridors
    pub fn generate_dungeon(width: i32, height: i32) -> GameMap {
        let mut game_map = GameMap::new(width, height);
        
        // Fill with walls initially
        for x in 0..width {
            for y in 0..height {
                game_map.tiles.insert((x, y), Tile::Wall);
            }
        }
        
        // Create a few simple rectangular rooms
        let rooms = vec![
            Room {
                x: 5,
                y: 5,
                width: 8,
                height: 6,
                id: 0,
                room_type: RoomType::Rectangle,
                is_illuminated: false,
                connected_rooms: vec![1],
            },
            Room {
                x: 20,
                y: 8,
                width: 10,
                height: 8,
                id: 1,
                room_type: RoomType::Rectangle,
                is_illuminated: false,
                connected_rooms: vec![0, 2],
            },
            Room {
                x: 15,
                y: 20,
                width: 6,
                height: 5,
                id: 2,
                room_type: RoomType::Rectangle,
                is_illuminated: false,
                connected_rooms: vec![1],
            },
        ];
        
        // Carve out rooms
        for room in &rooms {
            Self::carve_room(&mut game_map, room);
        }
        
        // Connect rooms with corridors
        Self::connect_rooms(&mut game_map, &rooms);
        
        // Add entrance and exit
        game_map.tiles.insert((6, 5), Tile::DungeonExit); // Entrance to room 0
        
        game_map.rooms = rooms;
        game_map
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
    
    /// Connect rooms with simple corridors
    fn connect_rooms(game_map: &mut GameMap, _rooms: &[Room]) {
        // Connect room 0 to room 1
        Self::carve_corridor(game_map, 13, 8, 20, 8); // Horizontal corridor
        
        // Connect room 1 to room 2  
        Self::carve_corridor(game_map, 25, 16, 25, 20); // Vertical corridor
        Self::carve_corridor(game_map, 21, 20, 25, 20); // Horizontal corridor
    }
    
    /// Carve a simple corridor between two points
    fn carve_corridor(game_map: &mut GameMap, x1: i32, y1: i32, x2: i32, y2: i32) {
        let mut x = x1;
        let mut y = y1;
        
        // Go horizontal first, then vertical
        while x != x2 {
            if x < x2 { x += 1; } else { x -= 1; }
            if x > 0 && y > 0 && x < game_map.width - 1 && y < game_map.height - 1 {
                game_map.tiles.insert((x, y), Tile::Corridor);
            }
        }
        
        while y != y2 {
            if y < y2 { y += 1; } else { y -= 1; }
            if x > 0 && y > 0 && x < game_map.width - 1 && y < game_map.height - 1 {
                game_map.tiles.insert((x, y), Tile::Corridor);
            }
        }
    }
    
    /// Generate a dungeon map based on entrance position for uniqueness
    pub fn generate_dungeon_for_entrance(entrance_x: i32, entrance_y: i32) -> GameMap {
        // Use entrance position as seed for consistent generation
        let seed = Self::generate_dungeon_seed(entrance_x, entrance_y);
        Self::generate_dungeon_with_seed(50, 30, seed) // Standard dungeon size
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
    /// Update player light and visibility
    pub fn update_lighting(&mut self, player_x: i32, player_y: i32, light_radius: i32) {
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
                    // Check line of sight
                    if self.has_line_of_sight(player_x, player_y, x, y) {
                        // Calculate brightness based on distance
                        let brightness = Self::calculate_brightness(distance, light_radius as f32);
                        
                        // Mark as visible if bright enough
                        if brightness > 0.1 {
                            self.visible_tiles.insert((x, y), true);
                            self.explored_tiles.insert((x, y), true);
                        }
                        
                        // Store light level for rendering
                        // We'll use a simple approach: store brightness in a separate map
                        // For now, we'll just mark as visible/invisible
                    }
                }
            }
        }
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
    
    /// Get the light level at a specific position
    pub fn get_light_level(&self, player_x: i32, player_y: i32, x: i32, y: i32, light_radius: i32) -> LightLevel {
        let dx = (x - player_x) as f32;
        let dy = (y - player_y) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        
        if distance <= light_radius as f32 && self.has_line_of_sight(player_x, player_y, x, y) {
            let brightness = Self::calculate_brightness(distance, light_radius as f32);
            LightLevel::new(brightness)
        } else {
            LightLevel::dark()
        }
    }
    
    /// Check if a tile should be rendered (visible or explored)
    pub fn should_render_tile(&self, x: i32, y: i32) -> bool {
        self.is_visible(x, y) || self.is_explored(x, y)
    }
    
    /// Get rendering style based on visibility and light level
    pub fn get_tile_visibility_state(&self, player_x: i32, player_y: i32, x: i32, y: i32, light_radius: i32) -> TileVisibility {
        if self.is_visible(x, y) {
            let light_level = self.get_light_level(player_x, player_y, x, y, light_radius);
            TileVisibility::Lit(light_level)
        } else if self.is_explored(x, y) {
            TileVisibility::Remembered
        } else {
            TileVisibility::Hidden
        }
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