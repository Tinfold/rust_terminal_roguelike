// Shared networking utilities to reduce duplication
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

/// Generic message sender type
pub type MessageSender<T> = mpsc::UnboundedSender<T>;

/// Generic message receiver type  
pub type MessageReceiver<T> = mpsc::UnboundedReceiver<T>;

/// Common networking utilities
pub struct NetworkUtils;

impl NetworkUtils {
    /// Serialize a message to JSON string
    pub fn serialize_message<T: Serialize>(message: &T) -> Result<String, serde_json::Error> {
        serde_json::to_string(message)
    }

    /// Deserialize a JSON string to message
    pub fn deserialize_message<T: for<'de> Deserialize<'de>>(json: &str) -> Result<T, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Create a message channel pair
    pub fn create_message_channel<T>() -> (MessageSender<T>, MessageReceiver<T>) {
        mpsc::unbounded_channel()
    }

    /// Safe send with error handling
    pub fn safe_send<T>(sender: &MessageSender<T>, message: T) -> bool {
        sender.send(message).is_ok()
    }
}

/// Common connection state
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connected: bool,
    pub last_activity: std::time::Instant,
    pub connection_id: String,
}

impl ConnectionState {
    pub fn new(connection_id: String) -> Self {
        Self {
            connected: true,
            last_activity: std::time::Instant::now(),
            connection_id,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}
