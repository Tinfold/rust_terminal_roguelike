use crate::common::terrain::{GameMap, Tile};
use image::{RgbImage, Rgb};
use std::path::Path;

pub struct DungeonVisualizer;

impl DungeonVisualizer {
    /// Generate and save a bitmap visualization of the dungeon
    pub fn save_dungeon_bitmap(dungeon: &GameMap, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Define colors for different tile types
        let wall_color = Rgb([64, 64, 64]);      // Dark gray
        let floor_color = Rgb([200, 200, 200]);  // Light gray
        let door_color = Rgb([139, 69, 19]);     // Brown
        let corridor_color = Rgb([150, 150, 200]); // Light blue
        let exit_color = Rgb([0, 255, 0]);       // Green
        let unknown_color = Rgb([255, 0, 0]);    // Red
        
        // Create a new image buffer with the dimensions of the dungeon
        let mut img = RgbImage::new(dungeon.width as u32, dungeon.height as u32);
        
        // Fill with black background
        for pixel in img.pixels_mut() {
            *pixel = Rgb([0, 0, 0]);
        }
        
        // Draw each tile
        for ((x, y), tile) in &dungeon.tiles {
            if *x >= 0 && *y >= 0 && *x < dungeon.width && *y < dungeon.height {
                let color = match tile {
                    Tile::Wall => wall_color,
                    Tile::Floor => {
                        // Check if this is part of a room (to color rooms differently)
                        if let Some(&room_id) = dungeon.room_positions.get(&(*x, *y)) {
                            // Generate unique color for each room
                            let r = ((room_id * 127) % 256) as u8;
                            let g = ((room_id * 191) % 256) as u8;
                            let b = ((room_id * 223) % 256) as u8;
                            Rgb([r, g, b])
                        } else {
                            floor_color
                        }
                    },
                    Tile::Corridor => corridor_color,
                    Tile::Door => door_color,
                    Tile::DungeonExit => exit_color,
                    _ => unknown_color,
                };
                
                img.put_pixel(*x as u32, *y as u32, color);
            }
        }
        
        // Create directories if they don't exist
        if let Some(parent) = Path::new(filename).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Save the image
        img.save(filename)?;
        println!("Dungeon visualization saved to: {}", filename);
        
        Ok(())
    }
}