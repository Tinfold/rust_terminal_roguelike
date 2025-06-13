use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

use rust_cli_roguelike::common::protocol::{
    ClientMessage, ServerMessage, GameState, NetworkPlayer, NetworkGameMap,
    NetworkCurrentScreen, PlayerId, MapType
};
use rust_cli_roguelike::common::game_logic::{GameLogic, Tile};

type SharedGameState = Arc<Mutex<ServerGameState>>;
type ClientSender = mpsc::UnboundedSender<ServerMessage>;
type ClientReceiver = mpsc::UnboundedReceiver<ServerMessage>;

#[derive(Debug)]
struct ServerGameState {
    players: HashMap<PlayerId, NetworkPlayer>,
    game_map: NetworkGameMap,
    current_map_type: MapType,
    turn_count: u32,
    client_senders: HashMap<PlayerId, ClientSender>,
}

impl ServerGameState {
    fn new() -> Self {
        let overworld = GameLogic::generate_overworld_map();
        let game_map = GameLogic::game_map_to_network(&overworld);

        Self {
            players: HashMap::new(),
            game_map,
            current_map_type: MapType::Overworld,
            turn_count: 0,
            client_senders: HashMap::new(),
        }
    }

    fn add_player(&mut self, player_id: PlayerId, player_name: String, sender: ClientSender) {
        let (spawn_x, spawn_y) = GameLogic::get_overworld_spawn_position();
        let player = NetworkPlayer {
            id: player_id.clone(),
            name: player_name,
            x: spawn_x,
            y: spawn_y,
            hp: 20,
            max_hp: 20,
            symbol: '@',
            current_screen: NetworkCurrentScreen::Game,
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

            // Check if the new position is valid
            if let Some(tile) = self.game_map.get_tile(new_x, new_y) {
                if GameLogic::is_movement_valid(*tile) {
                    player.x = new_x;
                    player.y = new_y;
                    self.turn_count += 1;

                    // Handle special tile interactions - send personalized messages to player
                    if let Some(interaction_message) = GameLogic::get_tile_interaction_message(*tile) {
                        let msg = ServerMessage::Message {
                            text: interaction_message,
                        };
                        // Send to the specific player
                        if let Some(sender) = self.client_senders.get(player_id) {
                            let _ = sender.send(msg);
                        }
                    }
                    
                    // Handle special multiplayer tile interactions - broadcast to all players
                    if *tile == Tile::Village {
                        let player_name = player.name.clone();
                        let msg = ServerMessage::Message {
                            text: format!("{} visits the village.", player_name),
                        };
                        self.broadcast_to_all(msg);
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
                    Err(GameLogic::get_blocked_movement_message(*tile))
                }
            } else {
                Err("Invalid position.".to_string())
            }
        } else {
            Err("Player not found.".to_string())
        }
    }

    fn enter_dungeon(&mut self, player_id: &PlayerId) -> Result<(), String> {
        if let Some(player) = self.players.get(player_id) {
            if GameLogic::is_at_network_dungeon_entrance(&self.game_map, player.x, player.y) {
                // Generate a new dungeon map using shared logic
                let dungeon = GameLogic::generate_dungeon_map();
                self.game_map = GameLogic::game_map_to_network(&dungeon);
                self.current_map_type = MapType::Dungeon;

                // Move all players to dungeon start
                let (spawn_x, spawn_y) = GameLogic::get_dungeon_spawn_position();
                for player in self.players.values_mut() {
                    player.x = spawn_x;
                    player.y = spawn_y;
                }

                self.broadcast_game_state();
                let msg = ServerMessage::Message {
                    text: "The party descends into the dungeon...".to_string(),
                };
                self.broadcast_to_all(msg);
                Ok(())
            } else {
                Err("You're not at a dungeon entrance.".to_string())
            }
        } else {
            Err("Player not found.".to_string())
        }
    }

    fn exit_dungeon(&mut self, _player_id: &PlayerId) -> Result<(), String> {
        if self.current_map_type == MapType::Dungeon {
            // Generate overworld using shared logic
            let overworld = GameLogic::generate_overworld_map();
            self.game_map = GameLogic::game_map_to_network(&overworld);
            self.current_map_type = MapType::Overworld;

            // Move all players back to overworld
            let (spawn_x, spawn_y) = GameLogic::get_overworld_spawn_position();
            for player in self.players.values_mut() {
                player.x = spawn_x;
                player.y = spawn_y;
            }

            self.broadcast_game_state();
            let msg = ServerMessage::Message {
                text: "The party emerges from the dungeon into the overworld.".to_string(),
            };
            self.broadcast_to_all(msg);
            Ok(())
        } else {
            Err("You're not in a dungeon.".to_string())
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
            game_map: self.game_map.clone(),
            current_map_type: self.current_map_type,
            turn_count: self.turn_count,
        };

        self.broadcast_to_all(ServerMessage::GameState { state: game_state });
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
