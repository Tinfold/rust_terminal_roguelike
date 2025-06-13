use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

use crate::protocol::{ClientMessage, ServerMessage};
use crate::app::NetworkClient;

impl NetworkClient {
    pub async fn connect(server_address: &str, player_name: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("ws://{}", server_address);
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        let (client_sender, mut client_receiver): (mpsc::UnboundedSender<ClientMessage>, _) = mpsc::unbounded_channel();
        let (server_sender, server_receiver): (mpsc::UnboundedSender<ServerMessage>, _) = mpsc::unbounded_channel();

        // Handle outgoing messages to server
        tokio::spawn(async move {
            while let Some(msg) = client_receiver.recv().await {
                let json = serde_json::to_string(&msg).unwrap();
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        });

        // Handle incoming messages from server
        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                            if server_sender.send(server_msg).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        });

        let client = Self {
            sender: client_sender,
            receiver: server_receiver,
            player_id: None,
            game_state: None,
            messages: Vec::new(),
            multiplayer_chunks: std::collections::HashMap::new(),
        };

        // Send initial connect message
        client.sender.send(ClientMessage::Connect { player_name })?;

        Ok(client)
    }
}
