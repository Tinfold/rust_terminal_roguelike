use std::collections::HashMap;
use noise::{NoiseFn, Perlin};

// Import types directly to avoid circular dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tile {
    Floor,
    Wall,
    Empty,
    // Overworld tiles
    Grass,
    Tree,
    Mountain,
    Water,
    Road,
    Village,
    DungeonEntrance,
}

#[derive(Debug, Clone)]
pub struct GameMap {
    pub width: i32,
    pub height: i32,
    pub tiles: HashMap<(i32, i32), Tile>,
}

pub struct TerrainGenerator;

impl TerrainGenerator {
    pub fn generate_overworld(width: i32, height: i32) -> GameMap {
        let mut game_map = GameMap {
            width,
            height,
            tiles: HashMap::new(),
        };
        
        // Create noise generators with different seeds for various terrain features
        let elevation_noise = Perlin::new(42);
        let moisture_noise = Perlin::new(123);
        let temperature_noise = Perlin::new(789);
        
        // Generate the base terrain using noise
        for x in 0..width {
            for y in 0..height {
                let tile = Self::generate_overworld_tile(
                    x, y, width, height, 
                    &elevation_noise, 
                    &moisture_noise,
                    &temperature_noise
                );
                game_map.tiles.insert((x, y), tile);
            }
        }
        
        // Generate rivers
        Self::generate_rivers(&mut game_map);
        
        // Add some villages and dungeon entrances
        Self::add_special_locations(&mut game_map);
        
        game_map
    }
    
    pub fn generate_dungeon(width: i32, height: i32) -> GameMap {
        let mut game_map = GameMap {
            width,
            height,
            tiles: HashMap::new(),
        };
        
        // Use cellular automata for more natural cave generation
        Self::generate_cave_dungeon(&mut game_map);
        
        game_map
    }
    
    fn generate_overworld_tile(
        x: i32, 
        y: i32, 
        _width: i32, 
        height: i32, 
        elevation_noise: &Perlin,
        moisture_noise: &Perlin,
        temperature_noise: &Perlin
    ) -> Tile {
        // Scale coordinates to get smoother noise
        let scale = 0.05;
        let scaled_x = x as f64 * scale;
        let scaled_y = y as f64 * scale;
        
        // Get elevation between -1 and 1, then normalize to 0-1
        let elevation = (elevation_noise.get([scaled_x, scaled_y]) + 1.0) / 2.0;
        
        // Get moisture with a different scale for variety
        let moisture = (moisture_noise.get([scaled_x * 0.7, scaled_y * 0.7]) + 1.0) / 2.0;
        
        // Get temperature with latitude influence (colder at top/bottom)
        let latitude_influence = 1.0 - ((y as f64 / height as f64 - 0.5) * 2.0).abs();
        let base_temp = (temperature_noise.get([scaled_x * 0.4, scaled_y * 0.4]) + 1.0) / 2.0;
        let temperature = base_temp * 0.7 + latitude_influence * 0.3;

        // Determine tile type based on elevation, moisture and temperature
        if elevation > 0.85 {
            // High mountains
            Tile::Mountain
        } else if elevation > 0.75 {
            // Hills and lower mountains
            if temperature < 0.3 {
                // Use Mountain instead of Snow (Snow is not in the Tile enum)
                Tile::Mountain 
            } else {
                Tile::Mountain
            }
        } else if elevation > 0.60 {
            // Hills and highlands
            if moisture > 0.6 {
                Tile::Tree
            } else {
                Tile::Grass
            }
        } else if elevation > 0.3 {
            // Regular terrain
            if moisture > 0.7 {
                Tile::Tree 
            } else if moisture > 0.4 {
                Tile::Grass
            } else {
                if temperature > 0.7 {
                    // Use Grass instead of Sand (Sand is not in the Tile enum)
                    Tile::Grass
                } else {
                    Tile::Grass
                }
            }
        } else {
            // Water bodies
            Tile::Water
        }
    }
    
    fn generate_rivers(game_map: &mut GameMap) {
        // Simple river generation
        let river_noise = Perlin::new(555);
        let river_count = game_map.width / 20 + 1; // Scale number of rivers with map size
        
        for i in 0..river_count {
            // Pick a starting point along the top edge
            let start_x = ((river_noise.get([i as f64 * 10.0, 0.0]) + 1.0) / 2.0 * game_map.width as f64) as i32;
            let mut x = start_x;
            let mut y = 0;
            
            // Flow the river down with some meandering
            while y < game_map.height {
                if let Some(tile) = game_map.tiles.get(&(x, y)) {
                    if *tile != Tile::Mountain {
                        game_map.tiles.insert((x, y), Tile::Water);
                    }
                }
                
                // Determine next direction with some noise-based meandering
                let dx_noise = river_noise.get([x as f64 * 0.1, y as f64 * 0.1, i as f64]);
                let dx = if dx_noise > 0.3 { 1 } else if dx_noise < -0.3 { -1 } else { 0 };
                
                // Always move down but sometimes sideways too
                y += 1;
                x = (x + dx).clamp(0, game_map.width - 1);
            }
        }
    }
    
    fn add_special_locations(game_map: &mut GameMap) {
        // Place villages near water but not on mountains or water
        let mut villages = Vec::new();
        let village_count = game_map.width / 15 + 2; // Scale number of villages with map size
        let village_noise = Perlin::new(888);
        
        for i in 0..village_count {
            let vx = ((village_noise.get([i as f64, 0.5]) + 1.0) / 2.0 * game_map.width as f64) as i32;
            let vy = ((village_noise.get([i as f64, 1.5]) + 1.0) / 2.0 * game_map.height as f64) as i32;
            
            // Check if position is suitable for a village
            if let Some(tile) = game_map.tiles.get(&(vx, vy)) {
                if *tile == Tile::Grass {
                    // Check if there's water nearby (good for villages)
                    let mut has_water_nearby = false;
                    for dx in -3..=3 {
                        for dy in -3..=3 {
                            if let Some(nearby) = game_map.tiles.get(&(vx + dx, vy + dy)) {
                                if *nearby == Tile::Water {
                                    has_water_nearby = true;
                                    break;
                                }
                            }
                        }
                    }
                    
                    if has_water_nearby {
                        game_map.tiles.insert((vx, vy), Tile::Village);
                        villages.push((vx, vy));
                    }
                }
            }
        }
        
        // Add dungeon entrances in interesting locations (near mountains, away from villages)
        let dungeon_count = village_count + 2; // More dungeons for better accessibility
        let dungeon_noise = Perlin::new(999);
        
        for i in 0..dungeon_count {
            let dx = ((dungeon_noise.get([i as f64, 10.5]) + 1.0) / 2.0 * game_map.width as f64) as i32;
            let dy = ((dungeon_noise.get([i as f64, 11.5]) + 1.0) / 2.0 * game_map.height as f64) as i32;
            
            // Check if position is suitable for a dungeon entrance
            if let Some(tile) = game_map.tiles.get(&(dx, dy)) {
                if *tile == Tile::Grass || *tile == Tile::Tree {
                    // Ensure it's not too close to villages
                    let mut too_close = false;
                    for (vx, vy) in &villages {
                        let distance = ((dx - vx).pow(2) + (dy - vy).pow(2)) as f32;
                        if distance < 100.0 { // arbitrary distance threshold
                            too_close = true;
                            break;
                        }
                    }
                    
                    if !too_close {
                        game_map.tiles.insert((dx, dy), Tile::DungeonEntrance);
                    }
                }
            }
        }
        
        // Add roads connecting villages and dungeons
        Self::add_roads(game_map);
    }
    
    fn add_roads(game_map: &mut GameMap) {
        // Find all villages and dungeons
        let mut important_locations = Vec::new();
        
        for x in 0..game_map.width {
            for y in 0..game_map.height {
                if let Some(tile) = game_map.tiles.get(&(x, y)) {
                    if *tile == Tile::Village || *tile == Tile::DungeonEntrance {
                        important_locations.push((x, y));
                    }
                }
            }
        }
        
        // Connect each location to its nearest neighbor
        for i in 0..important_locations.len() {
            let (x1, y1) = important_locations[i];
            let mut closest_idx = None;
            let mut closest_dist = f32::MAX;
            
            // Find closest other location
            for j in 0..important_locations.len() {
                if i == j { continue; }
                
                let (x2, y2) = important_locations[j];
                let dist = ((x2 - x1).pow(2) + (y2 - y1).pow(2)) as f32;
                
                if dist < closest_dist {
                    closest_dist = dist;
                    closest_idx = Some(j);
                }
            }
            
            // Draw road between locations using Bresenham's line algorithm
            if let Some(j) = closest_idx {
                let (x2, y2) = important_locations[j];
                Self::draw_road(game_map, x1, y1, x2, y2);
            }
        }
    }
    
    fn draw_road(game_map: &mut GameMap, x1: i32, y1: i32, x2: i32, y2: i32) {
        // Simple Bresenham's line algorithm for road drawing
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx - dy;
        
        let mut x = x1;
        let mut y = y1;
        
        loop {
            // Skip the endpoints (which are villages or dungeons)
            if (x != x1 || y != y1) && (x != x2 || y != y2) {
                if let Some(tile) = game_map.tiles.get(&(x, y)) {
                    // Don't draw roads over water or mountains
                    if *tile != Tile::Water && *tile != Tile::Mountain {
                        game_map.tiles.insert((x, y), Tile::Road);
                    }
                }
            }
            
            if x == x2 && y == y2 { break; }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
    
    fn generate_cave_dungeon(game_map: &mut GameMap) {
        // Initialize with random walls and floors
        let wall_chance = 0.4;
        let cave_noise = Perlin::new(123);
        
        for x in 0..game_map.width {
            for y in 0..game_map.height {
                // Always have walls on the border
                let tile = if x == 0 || x == game_map.width - 1 || y == 0 || y == game_map.height - 1 {
                    Tile::Wall
                } else {
                    // Use noise for initial cave generation
                    let noise_val = cave_noise.get([x as f64 * 0.1, y as f64 * 0.1]);
                    if noise_val < wall_chance * 2.0 - 1.0 {
                        Tile::Wall
                    } else {
                        Tile::Floor
                    }
                };
                
                game_map.tiles.insert((x, y), tile);
            }
        }
        
        // Apply cellular automata to create natural cave shapes
        for _ in 0..4 { // 4 iterations of smoothing
            let mut new_tiles = HashMap::new();
            
            for x in 0..game_map.width {
                for y in 0..game_map.height {
                    // Count neighboring walls
                    let mut walls = 0;
                    for nx in x-1..=x+1 {
                        for ny in y-1..=y+1 {
                            if nx == x && ny == y { continue; } // Skip center
                            
                            if let Some(tile) = game_map.tiles.get(&(nx, ny)) {
                                if *tile == Tile::Wall {
                                    walls += 1;
                                }
                            } else {
                                walls += 1; // Treat out-of-bounds as walls
                            }
                        }
                    }
                    
                    // Apply cellular automata rules
                    let new_tile = if game_map.tiles.get(&(x, y)) == Some(&Tile::Wall) {
                        if walls >= 4 { Tile::Wall } else { Tile::Floor }
                    } else {
                        if walls >= 5 { Tile::Wall } else { Tile::Floor }
                    };
                    
                    // Always keep walls on the border
                    if x == 0 || x == game_map.width - 1 || y == 0 || y == game_map.height - 1 {
                        new_tiles.insert((x, y), Tile::Wall);
                    } else {
                        new_tiles.insert((x, y), new_tile);
                    }
                }
            }
            
            // Update the game map with new tiles
            game_map.tiles = new_tiles;
        }
    }
}
