use std::collections::HashMap;
use noise::{NoiseFn, Perlin};
use serde::{Serialize, Deserialize};
use super::terrain::Tile;

/// Size of each chunk in tiles
pub const CHUNK_SIZE: i32 = 32;

/// Radius of chunks to keep loaded around the player
pub const CHUNK_LOAD_RADIUS: i32 = 3;

/// Maximum number of chunks to keep in memory
pub const MAX_LOADED_CHUNKS: usize = 64;

/// Represents a 2D coordinate for a chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32) -> Self {
        ChunkCoord { x, y }
    }

    /// Convert world coordinates to chunk coordinates
    pub fn from_world_pos(world_x: i32, world_y: i32) -> Self {
        ChunkCoord {
            x: world_x.div_euclid(CHUNK_SIZE),
            y: world_y.div_euclid(CHUNK_SIZE),
        }
    }

    /// Get the world coordinates of the chunk's top-left corner
    pub fn to_world_pos(&self) -> (i32, i32) {
        (self.x * CHUNK_SIZE, self.y * CHUNK_SIZE)
    }

    /// Get distance to another chunk coordinate
    pub fn distance_to(&self, other: &ChunkCoord) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs())
    }

    /// Get all chunk coordinates within a given radius
    pub fn neighbors_within_radius(&self, radius: i32) -> Vec<ChunkCoord> {
        let mut neighbors = Vec::new();
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                neighbors.push(ChunkCoord {
                    x: self.x + dx,
                    y: self.y + dy,
                });
            }
        }
        neighbors
    }
}

/// A chunk containing a fixed-size grid of tiles
#[derive(Debug, Clone)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub tiles: HashMap<(i32, i32), Tile>, // Local coordinates within chunk (0..CHUNK_SIZE)
    pub generated: bool,
    pub last_accessed: std::time::Instant,
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Chunk {
            coord,
            tiles: HashMap::new(),
            generated: false,
            last_accessed: std::time::Instant::now(),
        }
    }

    /// Get a tile at local chunk coordinates (0..CHUNK_SIZE)
    pub fn get_tile(&mut self, local_x: i32, local_y: i32) -> Option<Tile> {
        self.last_accessed = std::time::Instant::now();
        self.tiles.get(&(local_x, local_y)).copied()
    }

    /// Set a tile at local chunk coordinates
    pub fn set_tile(&mut self, local_x: i32, local_y: i32, tile: Tile) {
        self.last_accessed = std::time::Instant::now();
        self.tiles.insert((local_x, local_y), tile);
    }

    /// Convert world coordinates to local chunk coordinates
    pub fn world_to_local(world_x: i32, world_y: i32) -> (i32, i32) {
        (
            world_x.rem_euclid(CHUNK_SIZE),
            world_y.rem_euclid(CHUNK_SIZE)
        )
    }

    /// Generate the chunk's terrain
    pub fn generate(&mut self, terrain_generator: &InfiniteTerrainGenerator) {
        if self.generated {
            return;
        }

        let (world_x_start, world_y_start) = self.coord.to_world_pos();
        
        for local_x in 0..CHUNK_SIZE {
            for local_y in 0..CHUNK_SIZE {
                let world_x = world_x_start + local_x;
                let world_y = world_y_start + local_y;
                
                let tile = terrain_generator.generate_tile_at(world_x, world_y);
                self.tiles.insert((local_x, local_y), tile);
            }
        }

        self.generated = true;
        self.last_accessed = std::time::Instant::now();
    }
}

/// Manages infinite terrain generation using a chunking system
#[derive(Debug)]
pub struct InfiniteTerrainGenerator {
    elevation_noise: Perlin,
    moisture_noise: Perlin,
    temperature_noise: Perlin,
    feature_noise: Perlin,
    seed: u32,
}

impl InfiniteTerrainGenerator {
    pub fn new(seed: u32) -> Self {
        InfiniteTerrainGenerator {
            elevation_noise: Perlin::new(seed),
            moisture_noise: Perlin::new(seed.wrapping_add(1000)),
            temperature_noise: Perlin::new(seed.wrapping_add(2000)),
            feature_noise: Perlin::new(seed.wrapping_add(4000)),
            seed,
        }
    }

    /// Generate a single tile at the given world coordinates
    pub fn generate_tile_at(&self, world_x: i32, world_y: i32) -> Tile {
        // Scale coordinates for different features
        let scale = 0.02; // Base terrain scale
        let detail_scale = 0.1; // Fine detail scale
        
        let scaled_x = world_x as f64 * scale;
        let scaled_y = world_y as f64 * scale;
        let detail_x = world_x as f64 * detail_scale;
        let detail_y = world_y as f64 * detail_scale;

        // Generate base terrain values
        let elevation = self.sample_elevation(scaled_x, scaled_y);
        let moisture = self.sample_moisture(scaled_x * 0.7, scaled_y * 0.7);
        let temperature = self.sample_temperature(scaled_x * 0.4, scaled_y * 0.4, world_y);
        let detail = self.sample_detail(detail_x, detail_y);

        // Generate special features
        if self.should_place_village(world_x, world_y) {
            return Tile::Village;
        }

        if self.should_place_dungeon_entrance(world_x, world_y) {
            return Tile::DungeonEntrance;
        }

        // Generate roads
        if self.should_place_road(world_x, world_y) {
            return Tile::Road;
        }

        // Generate rivers
        if self.should_place_water(world_x, world_y, elevation) {
            return Tile::Water;
        }

        // Generate terrain based on elevation, moisture, and temperature
        self.determine_biome_tile(elevation, moisture, temperature, detail)
    }

    fn sample_elevation(&self, x: f64, y: f64) -> f64 {
        // Combine multiple octaves for more natural terrain
        let base = self.elevation_noise.get([x, y]);
        let detail = self.elevation_noise.get([x * 2.0, y * 2.0]) * 0.5;
        let fine = self.elevation_noise.get([x * 4.0, y * 4.0]) * 0.25;
        
        (base + detail + fine) * 0.5 + 0.5 // Normalize to 0-1
    }

    fn sample_moisture(&self, x: f64, y: f64) -> f64 {
        let base = self.moisture_noise.get([x, y]);
        let detail = self.moisture_noise.get([x * 3.0, y * 3.0]) * 0.3;
        
        (base + detail) * 0.5 + 0.5 // Normalize to 0-1
    }

    fn sample_temperature(&self, x: f64, y: f64, world_y: i32) -> f64 {
        // Base temperature from noise
        let base_temp = self.temperature_noise.get([x, y]);
        
        // Add latitude influence (equator is warmer)
        let latitude_factor = 1.0 - (world_y as f64 * 0.001).abs().min(1.0);
        
        (base_temp * 0.7 + latitude_factor * 0.3) * 0.5 + 0.5 // Normalize to 0-1
    }

    fn sample_detail(&self, x: f64, y: f64) -> f64 {
        self.feature_noise.get([x, y]) * 0.5 + 0.5
    }

    fn should_place_village(&self, world_x: i32, world_y: i32) -> bool {
        // Villages appear at specific pseudo-random locations
        let hash = self.hash_coords(world_x, world_y, 12345);
        hash % 10000 == 0 && self.is_suitable_for_village(world_x, world_y)
    }

    fn should_place_dungeon_entrance(&self, world_x: i32, world_y: i32) -> bool {
        // Dungeon entrances are rarer than villages
        let hash = self.hash_coords(world_x, world_y, 54321);
        hash % 15000 == 0 && self.is_suitable_for_dungeon(world_x, world_y)
    }

    fn should_place_road(&self, world_x: i32, world_y: i32) -> bool {
        // Create organic roads using noise instead of a grid pattern
        let road_noise = self.feature_noise.get([world_x as f64 * 0.008, world_y as f64 * 0.012]);
        let perpendicular_noise = self.feature_noise.get([world_y as f64 * 0.008, world_x as f64 * 0.012]);
        
        // Roads appear in areas where noise creates thin lines
        let road_threshold = 0.85;
        let road_width = 0.05;
        
        // Create horizontal-ish roads
        let horizontal_road = road_noise.abs() > road_threshold && 
                            (perpendicular_noise.abs() < road_width);
        
        // Create vertical-ish roads  
        let vertical_road = perpendicular_noise.abs() > road_threshold && 
                          (road_noise.abs() < road_width);
        
        // Only place roads in suitable terrain (not in water or mountains)
        if horizontal_road || vertical_road {
            let elevation = self.sample_elevation(world_x as f64 * 0.02, world_y as f64 * 0.02);
            elevation > 0.2 && elevation < 0.7
        } else {
            false
        }
    }

    fn should_place_water(&self, world_x: i32, world_y: i32, elevation: f64) -> bool {
        // Rivers follow low elevation paths
        if elevation > 0.4 {
            return false;
        }

        // Use noise to create winding rivers
        let river_noise = self.moisture_noise.get([world_x as f64 * 0.01, world_y as f64 * 0.05]);
        river_noise > 0.3 && elevation < 0.25
    }

    fn is_suitable_for_village(&self, world_x: i32, world_y: i32) -> bool {
        let elevation = self.sample_elevation(world_x as f64 * 0.02, world_y as f64 * 0.02);
        let moisture = self.sample_moisture(world_x as f64 * 0.014, world_y as f64 * 0.014);
        
        // Villages prefer moderate elevation and good moisture
        elevation > 0.3 && elevation < 0.7 && moisture > 0.4
    }

    fn is_suitable_for_dungeon(&self, world_x: i32, world_y: i32) -> bool {
        let elevation = self.sample_elevation(world_x as f64 * 0.02, world_y as f64 * 0.02);
        
        // Dungeons prefer higher elevation (mountains/hills)
        elevation > 0.6
    }

    fn determine_biome_tile(&self, elevation: f64, moisture: f64, temperature: f64, detail: f64) -> Tile {
        // High elevation = mountains
        if elevation > 0.8 {
            return Tile::Mountain;
        }

        // Very low elevation with high moisture = water
        if elevation < 0.2 && moisture > 0.6 {
            return Tile::Water;
        }

        // Medium-high elevation
        if elevation > 0.6 {
            if temperature < 0.3 {
                Tile::Mountain // Cold mountains
            } else if moisture > 0.5 {
                Tile::Tree // Forested hills
            } else {
                Tile::Mountain // Dry hills
            }
        }
        // Medium elevation
        else if elevation > 0.4 {
            if moisture > 0.6 {
                if detail > 0.7 {
                    Tile::Tree // Dense forest
                } else {
                    Tile::Grass // Forest edge
                }
            } else if moisture > 0.3 {
                Tile::Grass // Plains
            } else {
                Tile::Grass // Dry grassland
            }
        }
        // Low elevation
        else {
            if moisture > 0.7 {
                Tile::Water // Wetlands
            } else if moisture > 0.4 {
                Tile::Grass // Wet grasslands
            } else {
                Tile::Grass // Dry lowlands
            }
        }
    }

    fn hash_coords(&self, x: i32, y: i32, salt: u32) -> u32 {
        let mut hash = self.seed;
        hash = hash.wrapping_add(x as u32).wrapping_mul(73);
        hash = hash.wrapping_add(y as u32).wrapping_mul(37);
        hash = hash.wrapping_add(salt).wrapping_mul(17);
        hash
    }
}

/// Manages loaded chunks and provides infinite terrain
#[derive(Debug)]
pub struct ChunkManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    terrain_generator: InfiniteTerrainGenerator,
    player_chunk: ChunkCoord,
}

impl ChunkManager {
    pub fn new(seed: u32) -> Self {
        ChunkManager {
            chunks: HashMap::new(),
            terrain_generator: InfiniteTerrainGenerator::new(seed),
            player_chunk: ChunkCoord::new(0, 0),
        }
    }

    /// Update the player's position and manage chunk loading/unloading
    pub fn update_player_position(&mut self, player_x: i32, player_y: i32) {
        let new_player_chunk = ChunkCoord::from_world_pos(player_x, player_y);
        
        if new_player_chunk != self.player_chunk {
            self.player_chunk = new_player_chunk;
            self.load_chunks_around_player();
            self.unload_distant_chunks();
        }
    }

    /// Get a tile at world coordinates, generating chunks as needed
    pub fn get_tile(&mut self, world_x: i32, world_y: i32) -> Option<Tile> {
        let chunk_coord = ChunkCoord::from_world_pos(world_x, world_y);
        let (local_x, local_y) = Chunk::world_to_local(world_x, world_y);

        // Ensure chunk is loaded and generated
        self.ensure_chunk_loaded(chunk_coord);

        // Get tile from chunk
        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            chunk.get_tile(local_x, local_y)
        } else {
            None
        }
    }

    /// Set a tile at world coordinates (for player modifications)
    pub fn set_tile(&mut self, world_x: i32, world_y: i32, tile: Tile) {
        let chunk_coord = ChunkCoord::from_world_pos(world_x, world_y);
        let (local_x, local_y) = Chunk::world_to_local(world_x, world_y);

        // Ensure chunk is loaded
        self.ensure_chunk_loaded(chunk_coord);

        // Set tile in chunk
        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            chunk.set_tile(local_x, local_y, tile);
        }
    }

    /// Get all loaded chunks for rendering optimization
    pub fn get_loaded_chunks(&self) -> &HashMap<ChunkCoord, Chunk> {
        &self.chunks
    }

    /// Get tiles in a rectangular area (for efficient rendering)
    pub fn get_tiles_in_area(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> HashMap<(i32, i32), Tile> {
        let mut tiles = HashMap::new();

        for world_x in min_x..=max_x {
            for world_y in min_y..=max_y {
                if let Some(tile) = self.get_tile(world_x, world_y) {
                    tiles.insert((world_x, world_y), tile);
                }
            }
        }

        tiles
    }

    fn ensure_chunk_loaded(&mut self, chunk_coord: ChunkCoord) {
        if !self.chunks.contains_key(&chunk_coord) {
            let mut chunk = Chunk::new(chunk_coord);
            chunk.generate(&self.terrain_generator);
            self.chunks.insert(chunk_coord, chunk);
        }
    }

    fn load_chunks_around_player(&mut self) {
        let chunks_to_load = self.player_chunk.neighbors_within_radius(CHUNK_LOAD_RADIUS);
        
        for chunk_coord in chunks_to_load {
            self.ensure_chunk_loaded(chunk_coord);
        }
    }

    fn unload_distant_chunks(&mut self) {
        // Remove chunks that are too far from the player or if we have too many loaded
        let chunks_to_remove: Vec<ChunkCoord> = self.chunks
            .iter()
            .filter(|(coord, chunk)| {
                let distance = self.player_chunk.distance_to(coord);
                distance > CHUNK_LOAD_RADIUS + 1 || 
                chunk.last_accessed.elapsed().as_secs() > 300 // 5 minutes
            })
            .map(|(coord, _)| *coord)
            .collect();

        for coord in chunks_to_remove {
            self.chunks.remove(&coord);
        }

        // If still too many chunks, remove the oldest ones
        while self.chunks.len() > MAX_LOADED_CHUNKS {
            if let Some(oldest_coord) = self.chunks
                .iter()
                .min_by_key(|(_, chunk)| chunk.last_accessed)
                .map(|(coord, _)| *coord)
            {
                self.chunks.remove(&oldest_coord);
            } else {
                break;
            }
        }
    }
}
