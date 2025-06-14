use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

use rust_cli_roguelike::common::protocol::{
    ClientMessage, ServerMessage, GameState, NetworkPlayer, ChunkData,
    NetworkCurrentScreen, PlayerId, MapType
};
use rust_cli_roguelike::common::game_logic::{GameLogic, Tile, GameChunkManager, GameMap};
use rust_cli_roguelike::common::chunk::CHUNK_SIZE;

type SharedGameState = Arc<Mutex<ServerGameState>>;
type ClientSender = mpsc::UnboundedSender<ServerMessage>;
type ClientReceiver = mpsc::UnboundedReceiver<ServerMessage>;

// Player color palette - distinct colors for multiplayer
const PLAYER_COLORS: [(u8, u8, u8); 10] = [
    (255, 69, 0),   // Red-Orange
    (50, 205, 50),  // Lime Green
    (30, 144, 255), // Dodger Blue
    (255, 20, 147), // Deep Pink
    (255, 215, 0),  // Gold
    (138, 43, 226), // Blue Violet
    (0, 255, 255),  // Cyan
    (255, 165, 0),  // Orange
    (124, 252, 0),  // Lawn Green
    (255, 105, 180),// Hot Pink
];

#[derive(Debug)]
struct ServerGameState {
    players: HashMap<PlayerId, NetworkPlayer>,
    chunk_manager: GameChunkManager,
    turn_count: u32,
    client_senders: HashMap<PlayerId, ClientSender>,
    // Store generated dungeons keyed by entrance coordinates
    dungeons: HashMap<(i32, i32), GameMap>,
    // Note: current_map_type is now per-player, not global
}

impl ServerGameState {
    fn new() -> Self {
        // Create chunk manager with a fixed seed for consistent multiplayer worlds
        let seed = 12345; // Fixed seed ensures all players see the same world
        let chunk_manager = GameLogic::create_chunk_manager(seed);

        Self {
            players: HashMap::new(),
            chunk_manager,
            turn_count: 0,
            client_senders: HashMap::new(),
            dungeons: HashMap::new(),
        }
    }

    fn add_player(&mut self, player_id: PlayerId, player_name: String, sender: ClientSender) {
        let (spawn_x, spawn_y) = GameLogic::get_overworld_spawn_position();
        
        // Assign a color based on the number of existing players
        let color_index = self.players.len() % PLAYER_COLORS.len();
        let color = PLAYER_COLORS[color_index];
        
        let player = NetworkPlayer {
            id: player_id.clone(),
            name: player_name,
            x: spawn_x,
            y: spawn_y,
            hp: 20,
            max_hp: 20,
            symbol: '@',
            current_screen: NetworkCurrentScreen::Game,
            color,
            current_map_type: MapType::Overworld, // New players start in overworld
            dungeon_entrance_pos: None, // No dungeon entrance initially
        };

        self.players.insert(player_id.clone(), player.clone());
        self.client_senders.insert(player_id.clone(), sender);

        // Notify all other players about the new player
        let join_message = ServerMessage::PlayerJoined {
            player_id: player_id.clone(),
            player: player.clone(),
        };
        self.broadcast_to_others(&player_id, join_message);
    }

    fn remove_player(&mut self, player_id: &PlayerId) {
        self.players.remove(player_id);
        self.client_senders.remove(player_id);

        // Notify all other players
        let leave_message = ServerMessage::PlayerLeft {
            player_id: player_id.clone(),
        };
        self.broadcast_to_all(leave_message);
    }

    fn move_player(&mut self, player_id: &PlayerId, dx: i32, dy: i32) -> Result<(), String> {
        if let Some(player) = self.players.get_mut(player_id) {
            let new_x = player.x + dx;
            let new_y = player.y + dy;
            let current_map_type = player.current_map_type;

            // Validate movement based on player's current map type
            let (tile, is_valid) = if current_map_type == MapType::Dungeon {
                // In dungeons, use the stored dungeon map for proper validation
                let tile = if let Some((entrance_x, entrance_y)) = player.dungeon_entrance_pos {
                    let entrance_key = (entrance_x, entrance_y);
                    if let Some(dungeon_map) = self.dungeons.get(&entrance_key) {
                        dungeon_map.tiles.get(&(new_x, new_y)).cloned()
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let is_valid = tile.map_or(false, |t| GameLogic::is_movement_valid(t));
                (tile, is_valid)
            } else {
                // In overworld, use chunk manager
                self.chunk_manager.update_player_position(new_x, new_y);
                let tile = self.chunk_manager.get_tile(new_x, new_y);
                let is_valid = tile.map_or(true, |t| GameLogic::is_movement_valid(t));
                (tile, is_valid)
            };

            if is_valid {
                player.x = new_x;
                player.y = new_y;
                self.turn_count += 1;

                // Handle special tile interactions only in overworld
                if current_map_type == MapType::Overworld {
                    if let Some(tile) = tile {
                        if let Some(interaction_message) = GameLogic::get_tile_interaction_message(tile) {
                            let msg = ServerMessage::Message {
                                text: interaction_message,
                            };
                            // Send to the specific player
                            if let Some(sender) = self.client_senders.get(player_id) {
                                let _ = sender.send(msg);
                            }
                        }
                        
                        // Handle special multiplayer tile interactions - broadcast to all players
                        if tile == Tile::Village {
                            let player_name = player.name.clone();
                            let msg = ServerMessage::Message {
                                text: format!("{} visits the village.", player_name),
                            };
                            self.broadcast_to_all(msg);
                        }
                    }
                }

                // Notify all players about the movement
                let move_message = ServerMessage::PlayerMoved {
                    player_id: player_id.clone(),
                    x: new_x,
                    y: new_y,
                };
                self.broadcast_to_all(move_message);

                // Send updated game state
                self.broadcast_game_state();
                Ok(())
            } else {
                let tile = tile.unwrap_or(Tile::Wall);
                Err(GameLogic::get_blocked_movement_message(tile))
            }
        } else {
            Err("Player not found.".to_string())
        }
    }

    fn enter_dungeon(&mut self, player_id: &PlayerId) -> Result<(), String> {
        // First check if player exists and get their current state
        let (player_x, player_y, player_name, is_in_overworld) = {
            if let Some(player) = self.players.get(player_id) {
                (player.x, player.y, player.name.clone(), player.current_map_type == MapType::Overworld)
            } else {
                return Err("Player not found.".to_string());
            }
        };

        if !is_in_overworld {
            return Err("You're already in a dungeon.".to_string());
        }

        // Check if player is at a dungeon entrance
        if !GameLogic::is_at_chunk_dungeon_entrance(&mut self.chunk_manager, player_x, player_y) {
            return Err("You're not at a dungeon entrance.".to_string());
        }

        // Get or generate the dungeon for this entrance
        let entrance_key = (player_x, player_y);
        let dungeon_map = if let Some(existing_dungeon) = self.dungeons.get(&entrance_key) {
            // Use existing dungeon
            existing_dungeon.clone()
        } else {
            // Generate new dungeon and store it
            let new_dungeon = GameLogic::generate_dungeon_map_for_entrance(player_x, player_y);
            self.dungeons.insert(entrance_key, new_dungeon.clone());
            new_dungeon
        };

        // Now move the player to the dungeon
        if let Some(player) = self.players.get_mut(player_id) {
            // Store the entrance position before moving to dungeon
            player.dungeon_entrance_pos = Some((player_x, player_y));
            
            let (spawn_x, spawn_y) = GameLogic::get_safe_dungeon_spawn_position(&dungeon_map);
            player.x = spawn_x;
            player.y = spawn_y;
            player.current_map_type = MapType::Dungeon;

            // Send the dungeon map to the player
            let network_dungeon_map = GameLogic::game_map_to_network(&dungeon_map);
            self.send_to_player(player_id, ServerMessage::DungeonData { 
                dungeon_map: network_dungeon_map 
            });

            self.broadcast_game_state();
            let msg = ServerMessage::Message {
                text: format!("{} descends into the dungeon...", player_name),
            };
            self.broadcast_to_all(msg);
            Ok(())
        } else {
            Err("Player not found.".to_string())
        }
    }

    fn exit_dungeon(&mut self, player_id: &PlayerId) -> Result<(), String> {
        // First check if player exists and get their current state
        let (player_name, is_in_dungeon, player_x, player_y) = {
            if let Some(player) = self.players.get(player_id) {
                (player.name.clone(), player.current_map_type == MapType::Dungeon, player.x, player.y)
            } else {
                return Err("Player not found.".to_string());
            }
        };

        if !is_in_dungeon {
            return Err("You're not in a dungeon.".to_string());
        }

        // In multiplayer, we need to check if the player is at a dungeon exit position
        // Use the stored dungeon map to check the tile at player's position
        if let Some(player) = self.players.get(player_id) {
            if let Some((entrance_x, entrance_y)) = player.dungeon_entrance_pos {
                let entrance_key = (entrance_x, entrance_y);
                if let Some(dungeon_map) = self.dungeons.get(&entrance_key) {
                    if !GameLogic::is_at_dungeon_exit(dungeon_map, player_x, player_y) {
                        return Err("You must be at the dungeon entrance (marked with '<') to exit.".to_string());
                    }
                } else {
                    // Fallback: generate dungeon if not found (shouldn't happen)
                    let dungeon_map = GameLogic::generate_dungeon_map_for_entrance(entrance_x, entrance_y);
                    if !GameLogic::is_at_dungeon_exit(&dungeon_map, player_x, player_y) {
                        return Err("You must be at the dungeon entrance (marked with '<') to exit.".to_string());
                    }
                }
            }
        }

        // Now move the player to the overworld
        if let Some(player) = self.players.get_mut(player_id) {
            // Use stored entrance position or fall back to default spawn
            let (spawn_x, spawn_y) = player.dungeon_entrance_pos
                .unwrap_or_else(|| GameLogic::get_overworld_spawn_position());
            
            player.x = spawn_x;
            player.y = spawn_y;
            player.current_map_type = MapType::Overworld;
            player.dungeon_entrance_pos = None; // Clear the stored entrance position

            self.broadcast_game_state();
            let msg = ServerMessage::Message {
                text: format!("{} emerges from the dungeon into the overworld.", player_name),
            };
            self.broadcast_to_all(msg);
            Ok(())
        } else {
            Err("Player not found.".to_string())
        }
    }

    fn update_player_screen(&mut self, player_id: &PlayerId, screen: NetworkCurrentScreen) {
        if let Some(player) = self.players.get_mut(player_id) {
            player.current_screen = screen;
            self.broadcast_game_state();
        }
    }

    fn handle_chat_message(&mut self, player_id: &PlayerId, message: String) {
        if let Some(player) = self.players.get(player_id) {
            let chat_msg = ServerMessage::ChatMessage {
                player_name: player.name.clone(),
                message,
            };
            self.broadcast_to_all(chat_msg);
        }
    }

    fn broadcast_to_all(&self, message: ServerMessage) {
        for sender in self.client_senders.values() {
            let _ = sender.send(message.clone());
        }
    }

    fn broadcast_to_others(&self, exclude_player_id: &PlayerId, message: ServerMessage) {
        for (player_id, sender) in &self.client_senders {
            if player_id != exclude_player_id {
                let _ = sender.send(message.clone());
            }
        }
    }

    fn send_to_player(&self, player_id: &PlayerId, message: ServerMessage) {
        if let Some(sender) = self.client_senders.get(player_id) {
            let _ = sender.send(message);
        }
    }

    fn broadcast_game_state(&self) {
        let game_state = GameState {
            players: self.players.clone(),
            turn_count: self.turn_count,
        };

        self.broadcast_to_all(ServerMessage::GameState { state: game_state });
    }

    fn handle_chunk_request(&mut self, player_id: &PlayerId, chunk_coords: Vec<(i32, i32)>) {
        let mut chunk_data = Vec::new();
        
        for (chunk_x, chunk_y) in chunk_coords {
            // Get all tiles in this chunk from the chunk manager
            let chunk_start_x = chunk_x * CHUNK_SIZE;
            let chunk_start_y = chunk_y * CHUNK_SIZE;
            let chunk_end_x = chunk_start_x + CHUNK_SIZE - 1;
            let chunk_end_y = chunk_start_y + CHUNK_SIZE - 1;
            
            let tiles_in_chunk = self.chunk_manager.get_tiles_in_area(
                chunk_start_x, chunk_start_y, chunk_end_x, chunk_end_y
            );
            
            // Convert world coordinates to local chunk coordinates
            let mut chunk_tiles = std::collections::HashMap::new();
            for ((world_x, world_y), tile) in tiles_in_chunk {
                let local_x = world_x - chunk_start_x;
                let local_y = world_y - chunk_start_y;
                chunk_tiles.insert(format!("{},{}", local_x, local_y), tile);
            }
            
            chunk_data.push(ChunkData {
                chunk_x,
                chunk_y,
                tiles: chunk_tiles,
            });
        }
        
        // Send chunk data to the requesting player
        self.send_to_player(player_id, ServerMessage::ChunkData { chunks: chunk_data });
    }

    fn handle_dungeon_data_request(&mut self, player_id: &PlayerId) {
        if let Some(player) = self.players.get(player_id) {
            if player.current_map_type == MapType::Dungeon {
                if let Some((entrance_x, entrance_y)) = player.dungeon_entrance_pos {
                    let entrance_key = (entrance_x, entrance_y);
                    if let Some(dungeon_map) = self.dungeons.get(&entrance_key) {
                        let network_dungeon_map = GameLogic::game_map_to_network(dungeon_map);
                        self.send_to_player(player_id, ServerMessage::DungeonData { 
                            dungeon_map: network_dungeon_map 
                        });
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Starting roguelike server on 127.0.0.1:8080");
    
    let listener = TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind");
    let game_state = Arc::new(Mutex::new(ServerGameState::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection from: {}", addr);
        let game_state = Arc::clone(&game_state);
        tokio::spawn(handle_client(stream, game_state));
    }
}

async fn handle_client(stream: TcpStream, game_state: SharedGameState) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            println!("WebSocket connection error: {}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (client_sender, mut client_receiver): (ClientSender, ClientReceiver) = mpsc::unbounded_channel();
    let player_id = Uuid::new_v4().to_string();

    // Handle outgoing messages to client
    tokio::spawn(async move {
        while let Some(msg) = client_receiver.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if ws_sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    let mut state = game_state.lock().await;
                    
                    match client_msg {
                        ClientMessage::Connect { player_name } => {
                            state.add_player(player_id.clone(), player_name, client_sender.clone());
                            
                            // Send connection confirmation
                            let _ = client_sender.send(ServerMessage::Connected {
                                player_id: player_id.clone(),
                            });
                            
                            // Send initial game state
                            state.broadcast_game_state();
                        }
                        ClientMessage::Move { dx, dy } => {
                            match state.move_player(&player_id, dx, dy) {
                                Ok(_) => {}
                                Err(err) => {
                                    // Send blocked movement message as regular message to match single-player experience
                                    state.send_to_player(&player_id, ServerMessage::Message {
                                        text: err,
                                    });
                                }
                            }
                        }
                        ClientMessage::RequestChunks { chunks } => {
                            state.handle_chunk_request(&player_id, chunks);
                        }
                        ClientMessage::RequestDungeonData => {
                            state.handle_dungeon_data_request(&player_id);
                        }
                        ClientMessage::EnterDungeon => {
                            match state.enter_dungeon(&player_id) {
                                Ok(_) => {}
                                Err(err) => {
                                    state.send_to_player(&player_id, ServerMessage::Error {
                                        message: err,
                                    });
                                }
                            }
                        }
                        ClientMessage::ExitDungeon => {
                            match state.exit_dungeon(&player_id) {
                                Ok(_) => {}
                                Err(err) => {
                                    state.send_to_player(&player_id, ServerMessage::Error {
                                        message: err,
                                    });
                                }
                            }
                        }
                        ClientMessage::OpenInventory => {
                            state.update_player_screen(&player_id, NetworkCurrentScreen::Inventory);
                        }
                        ClientMessage::CloseInventory => {
                            state.update_player_screen(&player_id, NetworkCurrentScreen::Game);
                        }
                        ClientMessage::Chat { message } => {
                            state.handle_chat_message(&player_id, message);
                        }
                        ClientMessage::Disconnect => {
                            state.remove_player(&player_id);
                            break;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => {
                let mut state = game_state.lock().await;
                state.remove_player(&player_id);
                break;
            }
            _ => {}
        }
    }

    println!("Client disconnected: {}", player_id);
}
