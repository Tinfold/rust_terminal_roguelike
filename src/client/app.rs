use std::collections::HashMap;
use rust_cli_roguelike::common::protocol::{GameState, NetworkPlayer, PlayerId, ClientMessage, ServerMessage};
use rust_cli_roguelike::common::game_logic::{GameLogic, GameChunkManager};

// Re-export common types for use by other client modules
pub use rust_cli_roguelike::common::protocol::{CurrentScreen, MapType};
pub use rust_cli_roguelike::common::game_logic::{Tile, GameMap, Player};

// Forward declaration - the actual NetworkClient is defined in network.rs
pub struct NetworkClient {
    pub sender: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<ServerMessage>,
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
                crate::protocol::ServerMessage::ChatMessage { player_name, message } => {
                    // Store chat message separately from game messages
                    // This will be handled by the App struct
                    self.messages.push(format!("[CHAT] {}: {}", player_name, message));
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

    pub fn send_chat_message(&self, message: String) {
        let _ = self.sender.send(crate::protocol::ClientMessage::Chat { message });
    }

    pub fn send_open_chat(&self) {
        // Chat is a local UI state, no need to notify server
    }

    pub fn send_close_chat(&self) {
        // Chat is a local UI state, no need to notify server
    }

    pub fn disconnect(&self) {
        let _ = self.sender.send(crate::protocol::ClientMessage::Disconnect);
    }
}

pub struct App {
    pub current_screen: rust_cli_roguelike::common::protocol::CurrentScreen,
    pub should_quit: bool,
    pub player: rust_cli_roguelike::common::game_logic::Player,
    pub game_map: rust_cli_roguelike::common::game_logic::GameMap,
    pub chunk_manager: Option<GameChunkManager>, // For infinite terrain in single player
    pub messages: Vec<String>,
    pub turn_count: u32,
    pub current_map_type: rust_cli_roguelike::common::protocol::MapType,
    pub game_mode: GameMode,
    pub network_client: Option<NetworkClient>,
    pub other_players: HashMap<PlayerId, NetworkPlayer>,
    pub main_menu_state: MainMenuState,
    pub server_address: String,
    pub player_name: String,
    // Chat functionality
    pub chat_messages: Vec<(String, String)>, // (player_name, message)
    pub chat_input: String,
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
            chunk_manager: None,
            messages: vec!["Welcome! Select game mode from the menu.".to_string()],
            turn_count: 0,
            current_map_type: MapType::Overworld,
            game_mode: GameMode::SinglePlayer,
            network_client: None,
            other_players: HashMap::new(),
            main_menu_state: MainMenuState::new(),
            server_address: "127.0.0.1:8080".to_string(),
            player_name: "Player".to_string(),
            chat_messages: Vec::new(),
            chat_input: String::new(),
        }
    }

    pub fn start_single_player(&mut self) {
        self.game_mode = GameMode::SinglePlayer;
        self.current_screen = CurrentScreen::Game;
        // Initialize infinite terrain with chunk manager
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        self.chunk_manager = Some(GameLogic::create_chunk_manager(seed));
        // Keep the old game_map empty for multiplayer compatibility
        self.game_map = GameMap {
            width: 0,
            height: 0,
            tiles: HashMap::new(),
        };
        self.messages = vec!["Welcome to the infinite overworld! Explore and discover new terrain as you move.".to_string()];
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
        
        // Update messages and extract chat messages
        for message in &new_messages {
            if let Some(chat_part) = message.strip_prefix("[CHAT] ") {
                if let Some(colon_pos) = chat_part.find(": ") {
                    let player_name = chat_part[..colon_pos].to_string();
                    let chat_message = chat_part[colon_pos + 2..].to_string();
                    self.chat_messages.push((player_name, chat_message));
                    // Keep only the last 50 chat messages
                    if self.chat_messages.len() > 50 {
                        self.chat_messages.drain(0..self.chat_messages.len() - 50);
                    }
                } else {
                    self.messages.push(message.clone());
                }
            } else {
                self.messages.push(message.clone());
            }
        }
        
        // Keep only the last 10 messages using shared logic
        GameLogic::limit_messages(&mut self.messages, 10);
    }

    fn update_from_network_state(&mut self, state: &GameState) {
        // Update game map using shared logic
        self.game_map = GameLogic::network_map_to_game(&state.game_map);
        
        self.current_map_type = state.current_map_type;
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
                // Optimistic update: update local position immediately
                let new_x = self.player.x + dx;
                let new_y = self.player.y + dy;
                
                // Check if the move is valid locally first
                if let Some(tile) = self.game_map.tiles.get(&(new_x, new_y)) {
                    if GameLogic::is_movement_valid(*tile) {
                        // Update local position immediately for responsive feel
                        self.player.x = new_x;
                        self.player.y = new_y;
                        self.turn_count += 1;
                        
                        // Send move to server
                        if let Some(ref client) = self.network_client {
                            client.send_move(dx, dy);
                        }
                    } else {
                        self.messages.push(GameLogic::get_blocked_movement_message(*tile));
                    }
                } else {
                    // Send move anyway in case server has different map state
                    if let Some(ref client) = self.network_client {
                        client.send_move(dx, dy);
                    }
                }
            }
        }
    }

    fn move_player_single(&mut self, dx: i32, dy: i32) {
        let new_x = self.player.x + dx;
        let new_y = self.player.y + dy;
        
        // Use chunk manager if available (infinite terrain), otherwise use traditional map
        let tile = if let Some(ref mut chunk_manager) = self.chunk_manager {
            chunk_manager.get_tile(new_x, new_y)
        } else {
            self.game_map.tiles.get(&(new_x, new_y)).copied()
        };
        
        if let Some(tile) = tile {
            if GameLogic::is_movement_valid(tile) {
                self.player.x = new_x;
                self.player.y = new_y;
                self.turn_count += 1;
                
                // Add flavor text for tile interactions
                if let Some(message) = GameLogic::get_tile_interaction_message(tile) {
                    self.messages.push(message);
                }
            } else {
                self.messages.push(GameLogic::get_blocked_movement_message(tile));
            }
        } else {
            // Empty space - allow movement in infinite terrain
            if self.chunk_manager.is_some() {
                self.player.x = new_x;
                self.player.y = new_y;
                self.turn_count += 1;
            } else {
                self.messages.push("You can't move there.".to_string());
            }
        }
        
        // Keep only the last 10 messages
        GameLogic::limit_messages(&mut self.messages, 10);
    }
    
    pub fn enter_dungeon(&mut self) {
        match self.game_mode {
            GameMode::SinglePlayer => {
                // Check for dungeon entrance using chunk manager if available
                let at_entrance = if let Some(ref mut chunk_manager) = self.chunk_manager {
                    GameLogic::is_at_chunk_dungeon_entrance(chunk_manager, self.player.x, self.player.y)
                } else {
                    GameLogic::is_at_dungeon_entrance(&self.game_map, self.player.x, self.player.y)
                };
                
                if at_entrance {
                    // Switch to traditional dungeon map for now
                    // TODO: Could also implement infinite dungeon generation
                    self.game_map = GameLogic::generate_dungeon_map();
                    self.chunk_manager = None; // Disable chunk manager in dungeons
                    let (spawn_x, spawn_y) = GameLogic::get_dungeon_spawn_position();
                    self.player.x = spawn_x;
                    self.player.y = spawn_y;
                    self.current_map_type = MapType::Dungeon;
                    self.messages.push("You descend into the dungeon...".to_string());
                } else {
                    self.messages.push("You're not at a dungeon entrance.".to_string());
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
                    // Re-enable infinite terrain when returning to overworld
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as u32;
                    self.chunk_manager = Some(GameLogic::create_chunk_manager(seed));
                    
                    // Clear the old finite map
                    self.game_map = GameMap {
                        width: 0,
                        height: 0,
                        tiles: HashMap::new(),
                    };
                    
                    let (spawn_x, spawn_y) = GameLogic::get_overworld_spawn_position();
                    self.player.x = spawn_x;
                    self.player.y = spawn_y;
                    self.current_map_type = MapType::Overworld;
                    self.messages.push("You emerge from the dungeon into the infinite overworld.".to_string());
                } else {
                    self.messages.push("You're not in a dungeon.".to_string());
                }
            }
            GameMode::MultiPlayer => {
                if let Some(ref client) = self.network_client {
                    client.send_exit_dungeon();
                }
            }
        }    }
    
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

    pub fn open_chat(&mut self) {
        if self.game_mode == GameMode::MultiPlayer {
            self.current_screen = CurrentScreen::Chat;
            self.chat_input.clear();
            if let Some(ref client) = self.network_client {
                client.send_open_chat();
            }
        }
    }

    pub fn close_chat(&mut self) {
        self.current_screen = CurrentScreen::Game;
        self.chat_input.clear();
        if self.game_mode == GameMode::MultiPlayer {
            if let Some(ref client) = self.network_client {
                client.send_close_chat();
            }
        }
    }

    pub fn send_chat_message(&mut self) {
        if !self.chat_input.trim().is_empty() && self.game_mode == GameMode::MultiPlayer {
            if let Some(ref client) = self.network_client {
                client.send_chat_message(self.chat_input.clone());
            }
            self.chat_input.clear();
            self.close_chat();
        }
    }

    pub fn add_char_to_chat(&mut self, c: char) {
        if self.chat_input.len() < 100 { // Limit chat message length
            self.chat_input.push(c);
        }
    }

    pub fn remove_char_from_chat(&mut self) {
        self.chat_input.pop();
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

