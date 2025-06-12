use std::collections::HashMap;
use crate::app::{GameMap, Tile};

pub struct TerrainGenerator;

impl TerrainGenerator {
    pub fn generate_overworld(width: i32, height: i32) -> GameMap {
        let mut game_map = GameMap {
            width,
            height,
            tiles: HashMap::new(),
        };
        
        // Simple noise-like generation using position-based pseudo-random
        for x in 0..width {
            for y in 0..height {
                let tile = Self::generate_overworld_tile(x, y, width, height);
                game_map.tiles.insert((x, y), tile);
            }
        }
        
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
        
        // Initialize a simple dungeon with walls around the border
        for x in 0..game_map.width {
            for y in 0..game_map.height {
                let tile = if x == 0 || x == game_map.width - 1 || y == 0 || y == game_map.height - 1 {
                    Tile::Wall
                } else {
                    Tile::Floor
                };
                game_map.tiles.insert((x, y), tile);
            }
        }
        
        game_map
    }
    
    fn generate_overworld_tile(x: i32, y: i32, width: i32, height: i32) -> Tile {
        // Simple pseudo-random based on position
        let seed = (x * 73 + y * 37 + x * y * 17) % 100;
        
        // Distance from center for terrain variation
        let center_x = width / 2;
        let center_y = height / 2;
        let distance_from_center = ((x - center_x).abs() + (y - center_y).abs()) as f32;
        let max_distance = (width + height) as f32 / 4.0;
        let distance_factor = distance_from_center / max_distance;
        
        match seed {
            0..=40 => Tile::Grass,
            41..=60 => {
                if distance_factor > 0.7 {
                    Tile::Mountain
                } else {
                    Tile::Grass
                }
            }
            61..=75 => Tile::Tree,
            76..=85 => {
                if distance_factor < 0.3 {
                    Tile::Water
                } else {
                    Tile::Tree
                }
            }
            86..=90 => Tile::Mountain,
            91..=95 => Tile::Water,
            _ => Tile::Road,
        }
    }
    
    fn add_special_locations(game_map: &mut GameMap) {
        // Add a few villages
        let village_positions = vec![(10, 8), (45, 22), (25, 5)];
        for (x, y) in village_positions {
            if x < game_map.width && y < game_map.height {
                game_map.tiles.insert((x, y), Tile::Village);
            }
        }
        
        // Add dungeon entrances
        let dungeon_positions = vec![(15, 20), (40, 10), (50, 25), (8, 25)];
        for (x, y) in dungeon_positions {
            if x < game_map.width && y < game_map.height {
                game_map.tiles.insert((x, y), Tile::DungeonEntrance);
            }
        }
        
        // Add some roads connecting villages and dungeons
        Self::add_roads(game_map);
    }
    
    fn add_roads(game_map: &mut GameMap) {
        // Simple road from village to village
        for x in 10..=25 {
            if let Some(tile) = game_map.tiles.get(&(x, 8)) {
                if *tile == Tile::Grass {
                    game_map.tiles.insert((x, 8), Tile::Road);
                }
            }
        }
        
        // Road from village to dungeon
        for y in 8..=20 {
            if let Some(tile) = game_map.tiles.get(&(15, y)) {
                if *tile == Tile::Grass {
                    game_map.tiles.insert((15, y), Tile::Road);
                }
            }
        }
    }
}
