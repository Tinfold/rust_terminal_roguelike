use std::collections::HashMap;
use crate::terrain::TerrainGenerator;

pub struct App {
    pub current_screen: CurrentScreen,
    pub should_quit: bool,
    pub player: Player,
    pub game_map: GameMap,
    pub messages: Vec<String>,
    pub turn_count: u32,
    pub current_map_type: MapType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Game,
    Inventory,
    Exiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapType {
    Overworld,
    Dungeon,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub symbol: char,
}

#[derive(Debug, Clone)]
pub struct GameMap {
    pub width: i32,
    pub height: i32,
    pub tiles: HashMap<(i32, i32), Tile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl App {
    pub fn new() -> App {
        let game_map = TerrainGenerator::generate_overworld(60, 30);
        
        App {
            current_screen: CurrentScreen::Game,
            should_quit: false,
            player: Player {
                x: 30,
                y: 15,
                hp: 20,
                max_hp: 20,
                symbol: '@',
            },
            game_map,
            messages: vec!["Welcome to the overworld! Look for dungeons (D) to explore.".to_string()],
            turn_count: 0,
            current_map_type: MapType::Overworld,
        }
    }
    
    pub fn move_player(&mut self, dx: i32, dy: i32) {
        let new_x = self.player.x + dx;
        let new_y = self.player.y + dy;
        
        // Check if the new position is valid
        if let Some(tile) = self.game_map.tiles.get(&(new_x, new_y)) {
            match tile {
                Tile::Floor | Tile::Grass | Tile::Road | Tile::Tree => {
                    self.player.x = new_x;
                    self.player.y = new_y;
                    self.turn_count += 1;
                    if *tile == Tile::Tree {
                        self.messages.push("You push through the thick forest.".to_string());
                    }
                }
                Tile::Wall | Tile::Mountain => {
                    self.messages.push(format!("You can't move through {}.", 
                        match tile {
                            Tile::Wall => "a wall",
                            Tile::Mountain => "a mountain",
                            _ => "that",
                        }
                    ));
                }
                Tile::Water => {
                    self.messages.push("You can't swim across the water.".to_string());
                }
                Tile::Village => {
                    self.player.x = new_x;
                    self.player.y = new_y;
                    self.turn_count += 1;
                    self.messages.push("You visit the village. The locals greet you warmly.".to_string());
                }
                Tile::DungeonEntrance => {
                    self.player.x = new_x;
                    self.player.y = new_y;
                    self.turn_count += 1;
                    self.messages.push("You stand before a dark dungeon entrance. Press 'e' to enter.".to_string());
                }
                Tile::Empty => {}
            }
        }
        
        // Keep only the last 10 messages
        if self.messages.len() > 10 {
            self.messages.remove(0);
        }
    }
    
    pub fn enter_dungeon(&mut self) {
        if let Some(tile) = self.game_map.tiles.get(&(self.player.x, self.player.y)) {
            if *tile == Tile::DungeonEntrance {
                self.game_map = TerrainGenerator::generate_dungeon(40, 20);
                self.player.x = 5;
                self.player.y = 5;
                self.current_map_type = MapType::Dungeon;
                self.messages.push("You descend into the dungeon...".to_string());
            }
        }
    }
    
    pub fn exit_dungeon(&mut self) {
        if self.current_map_type == MapType::Dungeon {
            self.game_map = TerrainGenerator::generate_overworld(60, 30);
            self.player.x = 30;
            self.player.y = 15;
            self.current_map_type = MapType::Overworld;
            self.messages.push("You emerge from the dungeon into the overworld.".to_string());
        }
    }
}

impl Tile {
    pub fn to_char(self) -> char {
        match self {
            Tile::Floor => '.',
            Tile::Wall => '#',
            Tile::Empty => ' ',
            Tile::Grass => '"',
            Tile::Tree => 'T',
            Tile::Mountain => '^',
            Tile::Water => '~',
            Tile::Road => '+',
            Tile::Village => 'V',
            Tile::DungeonEntrance => 'D',
        }
    }
}