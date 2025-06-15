// Shared constants to reduce duplication across client and server
pub struct GameConstants;

impl GameConstants {
    // Map dimensions
    pub const OVERWORLD_WIDTH: i32 = 60;
    pub const OVERWORLD_HEIGHT: i32 = 30;
    pub const DUNGEON_WIDTH: i32 = 80;  // Ensure at least 60
    pub const DUNGEON_HEIGHT: i32 = 50; // Ensure at least 40

    // Spawn positions
    pub const OVERWORLD_SPAWN_X: i32 = 30;
    pub const OVERWORLD_SPAWN_Y: i32 = 15;
    pub const DUNGEON_SPAWN_X: i32 = 10;
    pub const DUNGEON_SPAWN_Y: i32 = 10;

    // Player stats
    pub const DEFAULT_HP: i32 = 20;
    pub const DEFAULT_MAX_HP: i32 = 20;
    pub const PLAYER_SYMBOL: char = '@';

    // UI constants
    pub const MAX_MESSAGES: usize = 10;
    pub const VIEWPORT_MIN_WIDTH: i32 = 60;
    pub const VIEWPORT_MIN_HEIGHT: i32 = 20;

    // Network constants
    pub const DEFAULT_SERVER_ADDRESS: &'static str = "127.0.0.1:8080";
    pub const DEFAULT_PLAYER_NAME: &'static str = "Player";
    pub const NETWORK_POLL_INTERVAL_MS: u64 = 50; // 20 FPS

    // Game messages
    pub const MSG_WELCOME_SINGLE: &'static str = "Welcome to the overworld! Look for dungeons (D) to explore.";
    pub const MSG_WELCOME_MULTI: &'static str = "Connected to multiplayer server!";
    pub const MSG_WELCOME_MENU: &'static str = "Welcome! Select game mode from the menu.";
    pub const MSG_ENTER_DUNGEON: &'static str = "You descend into the dungeon...";
    pub const MSG_EXIT_DUNGEON: &'static str = "You emerge from the dungeon into the overworld.";
    pub const MSG_ENTER_DUNGEON_PARTY: &'static str = "The party descends into the dungeon...";
    pub const MSG_EXIT_DUNGEON_PARTY: &'static str = "The party emerges from the dungeon into the overworld.";
    pub const MSG_NOT_AT_ENTRANCE: &'static str = "You're not at a dungeon entrance.";
    pub const MSG_NOT_IN_DUNGEON: &'static str = "You're not in a dungeon.";
    pub const MSG_PLAYER_NOT_FOUND: &'static str = "Player not found.";
    pub const MSG_INVALID_POSITION: &'static str = "Invalid position.";
    pub const MSG_CONNECTED: &'static str = "Connected to server!";
}
