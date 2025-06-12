use std::collections::HashMap;
use crate::terrain::TerrainGenerator;
use crate::protocol::{GameState, NetworkPlayer, PlayerId};

// Forward declaration - the actual NetworkClient is defined in network.rs
pub struct NetworkClient {
    pub sender: tokio::sync::mpsc::UnboundedSender<crate::protocol::ClientMessage>,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<crate::protocol::ServerMessage>,
    pub player_id: Option<PlayerId>,
    pub game_state: Option<GameState>,
    pub messages: Vec<String>,
}

impl NetworkClient {
    pub fn process_messages(&mut self) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                crate::protocol::ServerMessage::Connected { player_id } => {
                    self.player_id = Some(player_id);
                    self.messages.push("Connected to server!".to_string());
                }
                crate::protocol::ServerMessage::GameState { state } => {
                    self.game_state = Some(state);
                }
                crate::protocol::ServerMessage::PlayerMoved { .. } => {
                    // Game state will be updated in the next GameState message
                }
                crate::protocol::ServerMessage::PlayerJoined { player_id: _, player } => {
                    self.messages.push(format!("{} joined the game!", player.name));
                }
                crate::protocol::ServerMessage::PlayerLeft { player_id } => {
                    self.messages.push(format!("Player {} left the game!", player_id));
                }
                crate::protocol::ServerMessage::Error { message } => {
                    self.messages.push(format!("Error: {}", message));
                }
                crate::protocol::ServerMessage::Message { text } => {
                    self.messages.push(text);
                }
            }
        }

        // Keep only the last 10 messages
        if self.messages.len() > 10 {
            self.messages.drain(0..self.messages.len() - 10);
        }
    }

    pub fn send_move(&self, dx: i32, dy: i32) {
        let _ = self.sender.send(crate::protocol::ClientMessage::Move { dx, dy });
    }

    pub fn send_enter_dungeon(&self) {
        let _ = self.sender.send(crate::protocol::ClientMessage::EnterDungeon);
    }

    pub fn send_exit_dungeon(&self) {
        let _ = self.sender.send(crate::protocol::ClientMessage::ExitDungeon);
    }

    pub fn send_open_inventory(&self) {
        let _ = self.sender.send(crate::protocol::ClientMessage::OpenInventory);
    }

    pub fn send_close_inventory(&self) {
        let _ = self.sender.send(crate::protocol::ClientMessage::CloseInventory);
    }

    pub fn disconnect(&self) {
        let _ = self.sender.send(crate::protocol::ClientMessage::Disconnect);
    }
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub should_quit: bool,
    pub player: Player,
    pub game_map: GameMap,
    pub messages: Vec<String>,
    pub turn_count: u32,
    pub current_map_type: MapType,
    pub game_mode: GameMode,
    pub network_client: Option<NetworkClient>,
    pub other_players: HashMap<PlayerId, NetworkPlayer>,
    pub main_menu_state: MainMenuState,
    pub server_address: String,
    pub player_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    MainMenu,
    Game,
    Inventory,
    Exiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    SinglePlayer,
    MultiPlayer,
}

#[derive(Debug, Clone)]
pub struct MainMenuState {
    pub selected_option: usize,
    pub connecting: bool,
    pub connection_error: Option<String>,
}

impl MainMenuState {
    pub fn new() -> Self {
        Self {
            selected_option: 0,
            connecting: false,
            connection_error: None,
        }
    }
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
        App {
            current_screen: CurrentScreen::MainMenu,
            should_quit: false,
            player: Player {
                x: 30,
                y: 15,
                hp: 20,
                max_hp: 20,
                symbol: '@',
            },
            game_map: GameMap {
                width: 0,
                height: 0,
                tiles: HashMap::new(),
            },
            messages: vec!["Welcome! Select game mode from the menu.".to_string()],
            turn_count: 0,
            current_map_type: MapType::Overworld,
            game_mode: GameMode::SinglePlayer,
            network_client: None,
            other_players: HashMap::new(),
            main_menu_state: MainMenuState::new(),
            server_address: "127.0.0.1:8080".to_string(),
            player_name: "Player".to_string(),
        }
    }

    pub fn start_single_player(&mut self) {
        self.game_mode = GameMode::SinglePlayer;
        self.current_screen = CurrentScreen::Game;
        self.game_map = TerrainGenerator::generate_overworld(60, 30);
        self.messages = vec!["Welcome to the overworld! Look for dungeons (D) to explore.".to_string()];
    }

    pub fn start_multiplayer(&mut self, network_client: NetworkClient) {
        self.game_mode = GameMode::MultiPlayer;
        self.network_client = Some(network_client);
        self.current_screen = CurrentScreen::Game;
        self.messages = vec!["Connected to multiplayer server!".to_string()];
    }

    pub fn process_network_messages(&mut self) {
        let mut game_state_update = None;
        let mut new_messages = Vec::new();
        
        if let Some(ref mut client) = self.network_client {
            client.process_messages();
            
            // Collect updates without borrowing self
            if let Some(ref game_state) = client.game_state {
                game_state_update = Some(game_state.clone());
            }
            
            // Collect new messages
            new_messages.extend(client.messages.drain(..));
        }
        
        // Apply updates
        if let Some(state) = game_state_update {
            self.update_from_network_state(&state);
        }
        
        // Update messages
        self.messages.extend(new_messages);
        
        // Keep only the last 10 messages
        if self.messages.len() > 10 {
            self.messages.drain(0..self.messages.len() - 10);
        }
    }

    fn update_from_network_state(&mut self, state: &GameState) {
        // Update game map
        let mut tiles = HashMap::new();
        for (coord_str, network_tile) in &state.game_map.tiles {
            if let Some((x, y)) = crate::protocol::string_to_coord(coord_str) {
                tiles.insert((x, y), (*network_tile).into());
            }
        }
        
        self.game_map = GameMap {
            width: state.game_map.width,
            height: state.game_map.height,
            tiles,
        };
        
        self.current_map_type = state.current_map_type.into();
        self.turn_count = state.turn_count;
        
        // Update player position from network state
        if let Some(client) = &self.network_client {
            if let Some(player_id) = &client.player_id {
                if let Some(network_player) = state.players.get(player_id) {
                    self.player.x = network_player.x;
                    self.player.y = network_player.y;
                    self.player.hp = network_player.hp;
                    self.player.max_hp = network_player.max_hp;
                }
            }
        }
        
        // Update other players
        self.other_players.clear();
        if let Some(client) = &self.network_client {
            if let Some(player_id) = &client.player_id {
                for (id, player) in &state.players {
                    if id != player_id {
                        self.other_players.insert(id.clone(), player.clone());
                    }
                }
            }
        }
    }
    
    pub fn move_player(&mut self, dx: i32, dy: i32) {
        match self.game_mode {
            GameMode::SinglePlayer => {
                self.move_player_single(dx, dy);
            }
            GameMode::MultiPlayer => {
                if let Some(ref client) = self.network_client {
                    client.send_move(dx, dy);
                }
            }
        }
    }

    fn move_player_single(&mut self, dx: i32, dy: i32) {
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
        match self.game_mode {
            GameMode::SinglePlayer => {
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
            GameMode::MultiPlayer => {
                if let Some(ref client) = self.network_client {
                    client.send_enter_dungeon();
                }
            }
        }
    }
    
    pub fn exit_dungeon(&mut self) {
        match self.game_mode {
            GameMode::SinglePlayer => {
                if self.current_map_type == MapType::Dungeon {
                    self.game_map = TerrainGenerator::generate_overworld(60, 30);
                    self.player.x = 30;
                    self.player.y = 15;
                    self.current_map_type = MapType::Overworld;
                    self.messages.push("You emerge from the dungeon into the overworld.".to_string());
                }
            }
            GameMode::MultiPlayer => {
                if let Some(ref client) = self.network_client {
                    client.send_exit_dungeon();
                }
            }
        }
    }

    pub fn open_inventory(&mut self) {
        self.current_screen = CurrentScreen::Inventory;
        if self.game_mode == GameMode::MultiPlayer {
            if let Some(ref client) = self.network_client {
                client.send_open_inventory();
            }
        }
    }

    pub fn close_inventory(&mut self) {
        self.current_screen = CurrentScreen::Game;
        if self.game_mode == GameMode::MultiPlayer {
            if let Some(ref client) = self.network_client {
                client.send_close_inventory();
            }
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(ref client) = self.network_client {
            client.disconnect();
        }
        self.network_client = None;
        self.current_screen = CurrentScreen::MainMenu;
        self.main_menu_state = MainMenuState::new();
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