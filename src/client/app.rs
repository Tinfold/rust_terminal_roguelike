use std::collections::HashMap;
use rust_cli_roguelike::common::protocol::{GameState, NetworkPlayer, PlayerId, ClientMessage, ServerMessage};
use rust_cli_roguelike::common::game_logic::{GameLogic, GameChunkManager};

// Re-export common types for use by other client modules
pub use rust_cli_roguelike::common::protocol::{CurrentScreen, MapType};
pub use rust_cli_roguelike::common::game_logic::{Tile, GameMap, Player};

// Helper function to parse local coordinate strings like "0,0"
fn parse_local_coords(coord_str: &str) -> Result<(i32, i32), ()> {
    let parts: Vec<&str> = coord_str.split(',').collect();
    if parts.len() == 2 {
        if let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
            return Ok((x, y));
        }
    }
    Err(())
}

// Forward declaration - the actual NetworkClient is defined in network.rs
pub struct NetworkClient {
    pub sender: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<ServerMessage>,
    pub player_id: Option<PlayerId>,
    pub game_state: Option<GameState>,
    pub messages: Vec<String>,
    pub multiplayer_chunks: HashMap<(i32, i32), HashMap<(i32, i32), Tile>>, // For multiplayer chunk storage
    pub dungeon_map: Option<GameMap>, // Store the current dungeon map from server
}

impl NetworkClient {
    pub fn process_messages(&mut self) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                ServerMessage::Connected { player_id } => {
                    self.player_id = Some(player_id);
                    self.messages.push("Connected to server!".to_string());
                }
                ServerMessage::GameState { state } => {
                    self.game_state = Some(state);
                }
                ServerMessage::PlayerMoved { .. } => {
                    // Game state will be updated in the next GameState message
                }
                ServerMessage::PlayerJoined { player_id: _, player } => {
                    self.messages.push(format!("{} joined the game!", player.name));
                }
                ServerMessage::PlayerLeft { player_id } => {
                    self.messages.push(format!("Player {} left the game!", player_id));
                }
                ServerMessage::Error { message } => {
                    self.messages.push(format!("Error: {}", message));
                }
                ServerMessage::Message { text } => {
                    self.messages.push(text);
                }
                ServerMessage::ChatMessage { player_name, message } => {
                    // Store chat message separately from game messages
                    // This will be handled by the App struct
                    self.messages.push(format!("[CHAT] {}: {}", player_name, message));
                }
                ServerMessage::ChunkData { chunks } => {
                    // Handle received chunk data from server
                    for chunk in chunks {
                        let mut chunk_tiles = HashMap::new();
                        for (local_coord_str, tile) in chunk.tiles {
                            if let Ok(coords) = parse_local_coords(&local_coord_str) {
                                chunk_tiles.insert(coords, tile);
                            }
                        }
                        self.multiplayer_chunks.insert((chunk.chunk_x, chunk.chunk_y), chunk_tiles);
                    }
                }
                ServerMessage::DungeonData { dungeon_map } => {
                    // Convert NetworkGameMap to GameMap and store it
                    let game_map = GameLogic::network_map_to_game(&dungeon_map);
                    self.dungeon_map = Some(game_map);
                    self.messages.push("Received dungeon map from server".to_string());
                }
            }
        }

        // Keep only the last 10 messages
        if self.messages.len() > 10 {
            self.messages.drain(0..self.messages.len() - 10);
        }
    }

    pub fn send_move(&self, dx: i32, dy: i32) {
        let _ = self.sender.send(ClientMessage::Move { dx, dy });
    }

    pub fn send_enter_dungeon(&self) {
        let _ = self.sender.send(ClientMessage::EnterDungeon);
    }

    pub fn send_exit_dungeon(&self) {
        let _ = self.sender.send(ClientMessage::ExitDungeon);
    }

    pub fn send_open_inventory(&self) {
        let _ = self.sender.send(ClientMessage::OpenInventory);
    }

    pub fn send_close_inventory(&self) {
        let _ = self.sender.send(ClientMessage::CloseInventory);
    }

    pub fn send_chat_message(&self, message: String) {
        let _ = self.sender.send(ClientMessage::Chat { message });
    }

    pub fn send_open_chat(&self) {
        // Chat is a local UI state, no need to notify server
    }

    pub fn send_close_chat(&self) {
        // Chat is a local UI state, no need to notify server
    }

    pub fn disconnect(&self) {
        let _ = self.sender.send(ClientMessage::Disconnect);
    }

    pub fn request_chunks(&self, chunks: Vec<(i32, i32)>) {
        let _ = self.sender.send(ClientMessage::RequestChunks { chunks });
    }

    pub fn send_request_dungeon_data(&self) {
        let _ = self.sender.send(ClientMessage::RequestDungeonData);
    }
}

pub struct App {
    pub current_screen: rust_cli_roguelike::common::protocol::CurrentScreen,
    pub should_quit: bool,
    pub player: rust_cli_roguelike::common::game_logic::Player,
    pub game_map: rust_cli_roguelike::common::game_logic::GameMap,
    pub chunk_manager: Option<GameChunkManager>, // For infinite terrain in single player
    pub multiplayer_chunks: HashMap<(i32, i32), HashMap<(i32, i32), Tile>>, // For multiplayer chunk storage
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
    pub chat_input_mode: bool, // True when actively typing in the chat bar
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
    pub username_input_mode: bool,
    pub username_input: String,
}

impl MainMenuState {
    pub fn new() -> Self {
        Self {
            selected_option: 0,
            connecting: false,
            connection_error: None,
            username_input_mode: false,
            username_input: String::new(),
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
                dungeon_entrance_pos: None,
                opened_doors: std::collections::HashSet::new(),
                explored_rooms: std::collections::HashSet::new(),
            },
            game_map: GameMap {
                width: 0,
                height: 0,
                tiles: HashMap::new(),
                rooms: Vec::new(),
                room_positions: HashMap::new(),
                visible_tiles: HashMap::new(),
                explored_tiles: HashMap::new(),
                illuminated_areas: HashMap::new(),
            },
            chunk_manager: None,
            multiplayer_chunks: HashMap::new(),
            messages: vec!["Welcome! Select game mode from the menu.".to_string()],
            turn_count: 0,
            current_map_type: MapType::Overworld,
            game_mode: GameMode::SinglePlayer,
            network_client: None,
            other_players: HashMap::new(),
            main_menu_state: MainMenuState::new(),
            server_address: "127.0.0.1:8080".to_string(),
            player_name: format!("Player{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() % 10000), // Generate unique default name
            chat_messages: Vec::new(),
            chat_input: String::new(),
            chat_input_mode: false,
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
            rooms: Vec::new(),
            room_positions: HashMap::new(),
            visible_tiles: HashMap::new(),
            explored_tiles: HashMap::new(),
            illuminated_areas: HashMap::new(),
        };
        self.messages = vec!["Welcome to the infinite overworld! Explore and discover new terrain as you move.".to_string()];
    }

    pub fn start_multiplayer(&mut self, network_client: NetworkClient) {
        self.game_mode = GameMode::MultiPlayer;
        self.network_client = Some(network_client);
        self.current_screen = CurrentScreen::Game;
        self.messages = vec!["Connected to multiplayer server!".to_string()];
        
        // Request initial chunks around the player's spawn position
        self.request_chunks_around_player();
    }

    pub fn process_network_messages(&mut self) {
        let mut game_state_update = None;
        let mut new_messages = Vec::new();
        let mut dungeon_map_update = None;
        
        if let Some(ref mut client) = self.network_client {
            client.process_messages();
            
            // Collect updates without borrowing self
            if let Some(ref game_state) = client.game_state {
                game_state_update = Some(game_state.clone());
            }
            
            // Check for dungeon map update
            if let Some(ref dungeon_map) = client.dungeon_map {
                dungeon_map_update = Some(dungeon_map.clone());
                client.dungeon_map = None; // Clear it after taking
            }
            
            // Collect new messages
            new_messages.extend(client.messages.drain(..));
        }
        
        // Apply updates
        if let Some(state) = game_state_update {
            self.update_from_network_state(&state);
        }
        
        // Apply dungeon map update
        if let Some(dungeon_map) = dungeon_map_update {
            self.game_map = dungeon_map;
            self.chunk_manager = None; // Disable chunk manager in dungeons
            self.messages.push("Entered dungeon from multiplayer server".to_string());
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
        // Note: In the new chunk-based system, game map data comes via ChunkData messages
        // The GameState only contains player data and game metadata
        
        self.turn_count = state.turn_count;
        
        // Update player position and map type from network state
        if let Some(client) = &self.network_client {
            if let Some(player_id) = &client.player_id {
                if let Some(network_player) = state.players.get(player_id) {
                    let old_map_type = self.current_map_type;
                    let new_map_type = network_player.current_map_type;
                    
                    self.player.x = network_player.x;
                    self.player.y = network_player.y;
                    self.player.hp = network_player.hp;
                    self.player.max_hp = network_player.max_hp;
                    self.current_map_type = new_map_type;
                    
                    // Sync exploration data from NetworkPlayer to local Player (for dungeon visibility)
                    self.player.opened_doors = network_player.opened_doors.clone();
                    self.player.explored_rooms = network_player.explored_rooms.clone();
                    self.player.dungeon_entrance_pos = network_player.dungeon_entrance_pos;
                    
                    // Handle map transitions in multiplayer
                    if old_map_type != new_map_type {
                        match new_map_type {
                            MapType::Dungeon => {
                                // Generate dungeon map when entering
                                self.game_map = GameLogic::generate_dungeon_map();
                                self.chunk_manager = None; // Disable chunk manager in dungeons
                                self.messages.push("You descend into the dungeon...".to_string());
                            }
                            MapType::Overworld => {
                                // Re-enable chunk manager when returning to overworld
                                let seed = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as u32;
                                self.chunk_manager = Some(GameLogic::create_chunk_manager(seed));
                                
                                // Clear the old dungeon map
                                self.game_map = GameMap {
                                    width: 0,
                                    height: 0,
                                    tiles: HashMap::new(),
                                    rooms: Vec::new(),
                                    room_positions: HashMap::new(),
                                    visible_tiles: HashMap::new(),
                                    explored_tiles: HashMap::new(),
                                    illuminated_areas: HashMap::new(),
                                };
                                self.messages.push("You emerge from the dungeon into the overworld.".to_string());
                            }
                        }
                    }
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
                
                // Check if the move is valid based on current map type
                let tile = if self.current_map_type == MapType::Dungeon {
                    // In dungeons, use the dungeon map tiles
                    self.game_map.tiles.get(&(new_x, new_y)).copied()
                } else {
                    // In overworld, use multiplayer chunks first, then fall back to traditional map
                    self.get_multiplayer_tile(new_x, new_y).or_else(|| 
                        self.game_map.tiles.get(&(new_x, new_y)).copied()
                    )
                };
                
                if let Some(tile) = tile {
                    if GameLogic::is_movement_valid(tile) {
                        // Update local position immediately for responsive feel
                        self.player.x = new_x;
                        self.player.y = new_y;
                        self.turn_count += 1;
                        
                        // Update lighting if in dungeon (player has a light source)
                        if self.current_map_type == MapType::Dungeon {
                            const LIGHT_RADIUS: i32 = 6; // Player's light radius
                            self.game_map.update_lighting(new_x, new_y, LIGHT_RADIUS);
                        }
                        
                        // Send move to server
                        if let Some(ref client) = self.network_client {
                            client.send_move(dx, dy);
                        }
                        
                        // Request chunks around new position if needed (only in overworld)
                        if self.current_map_type == MapType::Overworld {
                            self.request_chunks_around_player();
                        }
                    } else {
                        self.messages.push(GameLogic::get_blocked_movement_message(tile));
                    }
                } else {
                    // Send move anyway in case server has different map state
                    if let Some(ref client) = self.network_client {
                        client.send_move(dx, dy);
                    }
                    
                    // Request chunks around new position
                    self.request_chunks_around_player();
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
                
                // Update lighting if in dungeon (player has a light source)
                if self.current_map_type == MapType::Dungeon {
                    const LIGHT_RADIUS: i32 = 6; // Player's light radius
                    self.game_map.update_lighting_with_doors(new_x, new_y, LIGHT_RADIUS, &self.player.opened_doors);
                }
                
                // Handle door opening in dungeons
                if self.current_map_type == MapType::Dungeon && tile == Tile::Door {
                    if GameLogic::open_door(&self.game_map, &mut self.player, new_x, new_y) {
                        self.messages.push("You open the door and reveal new areas!".to_string());
                        
                        // Update lighting again after opening door to reveal what's behind it
                        const LIGHT_RADIUS: i32 = 6;
                        self.game_map.update_lighting_with_doors(new_x, new_y, LIGHT_RADIUS, &self.player.opened_doors);
                    }
                }
                
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
                    // Store the entrance position before entering the dungeon
                    let entrance_pos = (self.player.x, self.player.y);
                    self.player.dungeon_entrance_pos = Some(entrance_pos);
                    
                    // Generate a unique dungeon based on entrance position
                    self.game_map = GameLogic::generate_dungeon_map_for_entrance(entrance_pos.0, entrance_pos.1);
                    self.chunk_manager = None; // Disable chunk manager in dungeons
                    let (spawn_x, spawn_y) = GameLogic::get_safe_dungeon_spawn_position(&self.game_map);
                    self.player.x = spawn_x;
                    self.player.y = spawn_y;
                    self.current_map_type = MapType::Dungeon;
                    
                    // Initialize player lighting in the dungeon with door awareness
                    const LIGHT_RADIUS: i32 = 6; // Player's light radius
                    self.game_map.update_lighting_with_doors(spawn_x, spawn_y, LIGHT_RADIUS, &self.player.opened_doors);
                    
                    // Initialize exploration system for the new dungeon
                    GameLogic::initialize_dungeon_exploration(&self.game_map, &mut self.player);
                    self.messages.push("You descend into the dungeon...".to_string());
                } else {
                    self.messages.push("You're not at a dungeon entrance.".to_string());
                }
            }
            GameMode::MultiPlayer => {
                if let Some(ref client) = self.network_client {
                    client.send_enter_dungeon();
                    // The server will automatically send dungeon data when we enter
                }
            }
        }
    }
    
    pub fn exit_dungeon(&mut self) {
        match self.game_mode {
            GameMode::SinglePlayer => {
                if self.current_map_type == MapType::Dungeon {
                    // Check if player is at a dungeon exit
                    if GameLogic::is_at_dungeon_exit(&self.game_map, self.player.x, self.player.y) {
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
                            rooms: Vec::new(),
                            room_positions: HashMap::new(),
                            visible_tiles: HashMap::new(),
                            explored_tiles: HashMap::new(),
                            illuminated_areas: HashMap::new(),
                        };
                        
                        // Use stored entrance position or fall back to default spawn
                        let (spawn_x, spawn_y) = self.player.dungeon_entrance_pos
                            .unwrap_or_else(|| GameLogic::get_overworld_spawn_position());
                        
                        self.player.x = spawn_x;
                        self.player.y = spawn_y;
                        self.player.dungeon_entrance_pos = None; // Clear the stored entrance position
                        self.current_map_type = MapType::Overworld;
                        self.messages.push("You emerge from the dungeon into the infinite overworld.".to_string());
                    } else {
                        self.messages.push("You must be at the dungeon entrance (marked with '<') to exit.".to_string());
                    }
                } else {
                    self.messages.push("You're not in a dungeon.".to_string());
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

    pub fn open_chat(&mut self) {
        if self.game_mode == GameMode::MultiPlayer {
            self.chat_input_mode = true;
            self.chat_input.clear();
        }
    }

    pub fn close_chat(&mut self) {
        self.chat_input_mode = false;
        self.chat_input.clear();
    }

    pub fn send_chat_message(&mut self) {
        if !self.chat_input.trim().is_empty() && self.game_mode == GameMode::MultiPlayer {
            if let Some(ref client) = self.network_client {
                client.send_chat_message(self.chat_input.clone());
            }
            self.chat_input.clear();
            self.chat_input_mode = false;
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

    // Username input methods
    pub fn start_username_input(&mut self) {
        self.main_menu_state.username_input_mode = true;
        self.main_menu_state.username_input = self.player_name.clone();
    }

    pub fn finish_username_input(&mut self) {
        if !self.main_menu_state.username_input.trim().is_empty() {
            self.player_name = self.main_menu_state.username_input.trim().to_string();
        }
        self.main_menu_state.username_input_mode = false;
        self.main_menu_state.username_input.clear();
    }

    pub fn cancel_username_input(&mut self) {
        self.main_menu_state.username_input_mode = false;
        self.main_menu_state.username_input.clear();
    }

    pub fn add_char_to_username(&mut self, c: char) {
        if self.main_menu_state.username_input.len() < 20 { // Limit username length
            self.main_menu_state.username_input.push(c);
        }
    }

    pub fn remove_char_from_username(&mut self) {
        self.main_menu_state.username_input.pop();
    }

    /// Get tile from multiplayer chunks (for chunk-based multiplayer terrain)
    pub fn get_multiplayer_tile(&self, x: i32, y: i32) -> Option<Tile> {
        if let Some(ref client) = self.network_client {
            // Calculate which chunk this position belongs to
            let chunk_x = if x >= 0 { x / 32 } else { (x - 31) / 32 };
            let chunk_y = if y >= 0 { y / 32 } else { (y - 31) / 32 };
            
            // Get local coordinates within the chunk
            let local_x = x - chunk_x * 32;
            let local_y = y - chunk_y * 32;
            
            // Check if we have this chunk
            if let Some(chunk_tiles) = client.multiplayer_chunks.get(&(chunk_x, chunk_y)) {
                return chunk_tiles.get(&(local_x, local_y)).copied();
            }
        }
        None
    }

    /// Request chunks around the player position from the server
    fn request_chunks_around_player(&mut self) {
        if let Some(ref client) = self.network_client {
            let player_chunk_x = if self.player.x >= 0 { self.player.x / 32 } else { (self.player.x - 31) / 32 };
            let player_chunk_y = if self.player.y >= 0 { self.player.y / 32 } else { (self.player.y - 31) / 32 };
            
            let mut chunks_to_request = Vec::new();
            
            // Request 3x3 grid of chunks around player
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let chunk_x = player_chunk_x + dx;
                    let chunk_y = player_chunk_y + dy;
                    
                    // Only request if we don't already have this chunk
                    if !client.multiplayer_chunks.contains_key(&(chunk_x, chunk_y)) {
                        chunks_to_request.push((chunk_x, chunk_y));
                    }
                }
            }
            
            if !chunks_to_request.is_empty() {
                client.request_chunks(chunks_to_request);
            }
        }
    }
}

