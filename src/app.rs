use std::collections::HashMap;

pub struct App {
    pub current_screen: CurrentScreen,
    pub should_quit: bool,
    pub player: Player,
    pub game_map: GameMap,
    pub messages: Vec<String>,
    pub turn_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Game,
    Inventory,
    Exiting,
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
}

impl App {
    pub fn new() -> App {
        let mut game_map = GameMap {
            width: 40,
            height: 20,
            tiles: HashMap::new(),
        };
        
        // Initialize a simple map with walls around the border
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
        
        App {
            current_screen: CurrentScreen::Game,
            should_quit: false,
            player: Player {
                x: 5,
                y: 5,
                hp: 20,
                max_hp: 20,
                symbol: '@',
            },
            game_map,
            messages: vec!["Welcome to the dungeon!".to_string()],
            turn_count: 0,
        }
    }
    
    pub fn move_player(&mut self, dx: i32, dy: i32) {
        let new_x = self.player.x + dx;
        let new_y = self.player.y + dy;
        
        // Check if the new position is valid
        if let Some(tile) = self.game_map.tiles.get(&(new_x, new_y)) {
            match tile {
                Tile::Floor => {
                    self.player.x = new_x;
                    self.player.y = new_y;
                    self.turn_count += 1;
                }
                Tile::Wall => {
                    self.messages.push("You bump into a wall.".to_string());
                }
                Tile::Empty => {}
            }
        }
        
        // Keep only the last 10 messages
        if self.messages.len() > 10 {
            self.messages.remove(0);
        }
    }
}

impl Tile {
    pub fn to_char(self) -> char {
        match self {
            Tile::Floor => '.',
            Tile::Wall => '#',
            Tile::Empty => ' ',
        }
    }
}