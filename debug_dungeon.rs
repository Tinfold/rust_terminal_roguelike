// Debug script to test dungeon generation
use std::collections::HashMap;

// Simplified structures for testing
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
pub struct GameMap {
    pub width: i32,
    pub height: i32,
    pub tiles: HashMap<(i32, i32), Tile>,
    pub rooms: Vec<Room>,
    pub room_positions: HashMap<(i32, i32), u32>,
    pub visible_tiles: HashMap<(i32, i32), bool>,
    pub explored_tiles: HashMap<(i32, i32), bool>,
    pub illuminated_areas: HashMap<u32, bool>,
}

impl GameMap {
    pub fn new(width: i32, height: i32) -> Self {
        GameMap {
            width,
            height,
            tiles: HashMap::new(),
            rooms: Vec::new(),
            room_positions: HashMap::new(),
            visible_tiles: HashMap::new(),
            explored_tiles: HashMap::new(),
            illuminated_areas: HashMap::new(),
        }
    }
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
        println!("Creating BSPNode: x={}, y={}, width={}, height={}, id={}", x, y, width, height, id);
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
            println!("Node {} cannot split", self.id);
            return false;
        }

        let split_horizontal = if self.width > self.height {
            false
        } else if self.height > self.width {
            true
        } else {
            *next_id % 2 == 0
        };

        println!("Node {} splitting {} (width={}, height={})", 
                self.id, if split_horizontal { "horizontally" } else { "vertically" }, 
                self.width, self.height);

        let (max_split, min_split_size) = if split_horizontal {
            (self.height - min_size, min_size)
        } else {
            (self.width - min_size, min_size)
        };

        if max_split <= min_split_size {
            println!("Node {} split failed: max_split={}, min_split_size={}", 
                    self.id, max_split, min_split_size);
            return false;
        }

        let split_pos = min_split_size + ((*next_id * 17) % (max_split - min_split_size) as u32) as i32;
        println!("Node {} split_pos={}", self.id, split_pos);

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

        println!("Node {} successfully split into {} and {}", 
                self.id, 
                self.left.as_ref().unwrap().id, 
                self.right.as_ref().unwrap().id);
        true
    }

    fn create_rooms(&mut self, min_room_size: i32, max_room_size: i32) {
        if let (Some(ref mut left), Some(ref mut right)) = (&mut self.left, &mut self.right) {
            println!("Node {} is internal, recursing to children", self.id);
            left.create_rooms(min_room_size, max_room_size);
            right.create_rooms(min_room_size, max_room_size);
        } else {
            println!("Node {} is leaf, creating room", self.id);
            let margin = 2;
            let max_width = (self.width - margin * 2).min(max_room_size);
            let max_height = (self.height - margin * 2).min(max_room_size);
            
            println!("Node {} room constraints: max_width={}, max_height={}", 
                    self.id, max_width, max_height);
            
            if max_width >= min_room_size && max_height >= min_room_size {
                let room_width = min_room_size + (self.id * 13) as i32 % (max_width - min_room_size + 1);
                let room_height = min_room_size + (self.id * 19) as i32 % (max_height - min_room_size + 1);
                
                let room_x = self.x + margin + (self.id * 23) as i32 % (self.width - room_width - margin * 2 + 1);
                let room_y = self.y + margin + (self.id * 29) as i32 % (self.height - room_height - margin * 2 + 1);
                
                println!("Node {} created room: x={}, y={}, width={}, height={}", 
                        self.id, room_x, room_y, room_width, room_height);
                
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
            } else {
                println!("Node {} cannot create room (too small)", self.id);
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
    println!("Splitting node {} at depth {}", node.id, 6 - max_depth);
    
    if max_depth <= 0 || !node.can_split(min_size) {
        println!("Node {} stopping split (depth={}, can_split={})", 
                node.id, max_depth, node.can_split(min_size));
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
    println!("Testing BSP Dungeon Generation");
    println!("==============================");
    
    let width = 80;
    let height = 40;
    
    println!("Dungeon size: {}x{}", width, height);
    
    // Create BSP tree
    let mut root = BSPNode::new(1, 1, width - 2, height - 2, 0);
    let mut next_id = 1;
    
    println!("\nStarting BSP split...");
    // Split the space recursively
    split_node_recursive(&mut root, &mut next_id, 8, 6);
    
    println!("\nCreating rooms...");
    // Create rooms in leaf nodes
    root.create_rooms(4, 12);
    
    println!("\nGetting all rooms...");
    // Get all rooms
    let mut rooms = Vec::new();
    root.get_rooms(&mut rooms);
    
    println!("\nGenerated {} rooms:", rooms.len());
    for room in &rooms {
        println!("Room {}: ({}, {}) {}x{}", 
                room.id, room.x, room.y, room.width, room.height);
    }
}
