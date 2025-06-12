use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

mod app;
mod terrain;
mod protocol;

use protocol::{
    ClientMessage, ServerMessage, GameState, NetworkPlayer, NetworkGameMap, NetworkTile,
    NetworkMapType, NetworkCurrentScreen, PlayerId
};
use terrain::TerrainGenerator;

type SharedGameState = Arc<Mutex<ServerGameState>>;
type ClientSender = mpsc::UnboundedSender<ServerMessage>;
type ClientReceiver = mpsc::UnboundedReceiver<ServerMessage>;

#[derive(Debug)]
struct ServerGameState {
    players: HashMap<PlayerId, NetworkPlayer>,
    game_map: NetworkGameMap,
    current_map_type: NetworkMapType,
    turn_count: u32,
    client_senders: HashMap<PlayerId, ClientSender>,
}

impl ServerGameState {
    fn new() -> Self {
        let overworld = TerrainGenerator::generate_overworld(60, 30);
        let network_tiles: HashMap<String, NetworkTile> = overworld.tiles
            .into_iter()
            .map(|(pos, tile)| (protocol::coord_to_string(pos.0, pos.1), tile.into()))
            .collect();

        Self {
            players: HashMap::new(),
            game_map: NetworkGameMap {
                width: overworld.width,
                height: overworld.height,
                tiles: network_tiles,
            },
            current_map_type: NetworkMapType::Overworld,
            turn_count: 0,
            client_senders: HashMap::new(),
        }
    }

    fn add_player(&mut self, player_id: PlayerId, player_name: String, sender: ClientSender) {
        let player = NetworkPlayer {
            id: player_id.clone(),
            name: player_name,
            x: 30,
            y: 15,
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
                match tile {
                    NetworkTile::Floor | NetworkTile::Grass | NetworkTile::Road | NetworkTile::Tree => {
                        player.x = new_x;
                        player.y = new_y;
                        self.turn_count += 1;

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
                    }
                    NetworkTile::Wall | NetworkTile::Mountain => {
                        Err(format!("Can't move through {}.", 
                            match tile {
                                NetworkTile::Wall => "a wall",
                                NetworkTile::Mountain => "a mountain",
                                _ => "that",
                            }
                        ))
                    }
                    NetworkTile::Water => {
                        Err("You can't swim across the water.".to_string())
                    }
                    NetworkTile::Village => {
                        player.x = new_x;
                        player.y = new_y;
                        self.turn_count += 1;
                        
                        let player_name = player.name.clone(); // Clone the name before other borrows
                        
                        let move_message = ServerMessage::PlayerMoved {
                            player_id: player_id.clone(),
                            x: new_x,
                            y: new_y,
                        };
                        self.broadcast_to_all(move_message);
                        self.broadcast_game_state();
                        
                        let msg = ServerMessage::Message {
                            text: format!("{} visits the village.", player_name),
                        };
                        self.broadcast_to_all(msg);
                        Ok(())
                    }
                    NetworkTile::DungeonEntrance => {
                        player.x = new_x;
                        player.y = new_y;
                        self.turn_count += 1;
                        
                        let move_message = ServerMessage::PlayerMoved {
                            player_id: player_id.clone(),
                            x: new_x,
                            y: new_y,
                        };
                        self.broadcast_to_all(move_message);
                        self.broadcast_game_state();
                        Ok(())
                    }
                    NetworkTile::Empty => Ok(()),
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
            if let Some(tile) = self.game_map.get_tile(player.x, player.y) {
                if *tile == NetworkTile::DungeonEntrance {
                    // For simplicity, generate a new dungeon map (in a real game, you'd manage multiple map instances)
                    let dungeon = TerrainGenerator::generate_dungeon(40, 20);
                    let network_tiles: HashMap<String, NetworkTile> = dungeon.tiles
                        .into_iter()
                        .map(|(pos, tile)| (protocol::coord_to_string(pos.0, pos.1), tile.into()))
                        .collect();

                    self.game_map = NetworkGameMap {
                        width: dungeon.width,
                        height: dungeon.height,
                        tiles: network_tiles,
                    };
                    self.current_map_type = NetworkMapType::Dungeon;

                    // Move all players to dungeon start
                    for player in self.players.values_mut() {
                        player.x = 5;
                        player.y = 5;
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
                Err("Invalid position.".to_string())
            }
        } else {
            Err("Player not found.".to_string())
        }
    }

    fn exit_dungeon(&mut self, _player_id: &PlayerId) -> Result<(), String> {
        if self.current_map_type == NetworkMapType::Dungeon {
            let overworld = TerrainGenerator::generate_overworld(60, 30);
            let network_tiles: HashMap<String, NetworkTile> = overworld.tiles
                .into_iter()
                .map(|(pos, tile)| (protocol::coord_to_string(pos.0, pos.1), tile.into()))
                .collect();

            self.game_map = NetworkGameMap {
                width: overworld.width,
                height: overworld.height,
                tiles: network_tiles,
            };
            self.current_map_type = NetworkMapType::Overworld;

            // Move all players back to overworld
            for player in self.players.values_mut() {
                player.x = 30;
                player.y = 15;
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
                                    state.send_to_player(&player_id, ServerMessage::Error {
                                        message: err,
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
