// Test with 50x30 dungeon size
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Door,
    DungeonExit,
    Corridor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomType {
    Rectangle,
}

#[derive(Debug, Clone)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub id: u32,
    pub room_type: RoomType,
    pub is_illuminated: bool,
    pub connected_rooms: Vec<u32>,
}

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

    fn can_split(&self, min_size: i32) -> bool {
        let can_split = self.width > min_size * 2 || self.height > min_size * 2;
        println!("Node {} can_split: {} (width={}, height={}, min_size={})", 
                self.id, can_split, self.width, self.height, min_size);
        can_split
    }

    fn split(&mut self, next_id: &mut u32, min_size: i32) -> bool {
        if !self.can_split(min_size) {
            return false;
        }

        let split_horizontal = if self.width > self.height {
            false
        } else if self.height > self.width {
            true
        } else {
            *next_id % 2 == 0
        };

        let (max_split, min_split_size) = if split_horizontal {
            (self.height - min_size, min_size)
        } else {
            (self.width - min_size, min_size)
        };

        if max_split <= min_split_size {
            return false;
        }

        let split_pos = min_split_size + ((*next_id * 17) % (max_split - min_split_size) as u32) as i32;

        if split_horizontal {
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

    fn create_rooms(&mut self, min_room_size: i32, max_room_size: i32) {
        if let (Some(ref mut left), Some(ref mut right)) = (&mut self.left, &mut self.right) {
            left.create_rooms(min_room_size, max_room_size);
            right.create_rooms(min_room_size, max_room_size);
        } else {
            let margin = 2;
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
}

fn split_node_recursive(node: &mut BSPNode, next_id: &mut u32, min_size: i32, max_depth: i32) {
    if max_depth <= 0 || !node.can_split(min_size) {
        return;
    }
    
    if node.split(next_id, min_size) {
        if let Some(ref mut left) = node.left {
            split_node_recursive(left, next_id, min_size, max_depth - 1);
        }
        if let Some(ref mut right) = node.right {
            split_node_recursive(right, next_id, min_size, max_depth - 1);
        }
    }
}

fn main() {
    println!("Testing 50x30 Dungeon with Current BSP Parameters");
    println!("================================================");
    
    let width = 50;
    let height = 30;
    
    // Create BSP tree
    let mut root = BSPNode::new(1, 1, width - 2, height - 2, 0);
    let mut next_id = 1;
    
    println!("Root node: {}x{}", root.width, root.height);
    
    // Use the same parameters as the main code
    split_node_recursive(&mut root, &mut next_id, 6, 5); // min_size=6, depth=5
    
    // Create rooms
    root.create_rooms(3, 8); // min_room_size=3, max_room_size=8
    
    let mut rooms = Vec::new();
    root.get_rooms(&mut rooms);
    
    println!("\nGenerated {} rooms:", rooms.len());
    for room in &rooms {
        println!("Room {}: ({}, {}) {}x{}", 
                room.id, room.x, room.y, room.width, room.height);
    }
}
