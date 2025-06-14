# Multiplayer CLI Roguelike

A terminal-based roguelike game with both single-player and multiplayer support, built in Rust using Ratatui for the UI and WebSockets for networking.

## Features

### Single Player Mode
- Explore an overworld with various terrain types (grass, trees, mountains, water, roads, villages)
- Enter and explore dungeons
- Traditional roguelike movement (HJKL keys or arrow keys)
- Inventory system
- Turn-based gameplay

### Multiplayer Mode
- Server-client architecture using WebSockets
- Multiple players can join the same game world
- Real-time synchronization of player positions and actions
- Shared world state (entering/exiting dungeons affects all players)
- Independent inventory management per player

## Quick Start

### Running the Server
```bash
cargo run --bin server
```
The server will start on `127.0.0.1:8080` and display connection status.

### Running the Client
```bash
cargo run --bin client
```
This opens the main menu where you can choose:
- **Single Player**: Play offline
- **Multiplayer**: Connect to a server at 127.0.0.1:8080
- **Quit**: Exit the game

### Testing Multiplayer
1. Start the server: `cargo run --bin server`
2. Open multiple terminals and run: `cargo run --bin client`
3. Select "Multiplayer" from the main menu in each client
4. You should see other players as cyan `@` symbols and the map will be visible
5. Move around with HJKL/arrows and watch real-time synchronization!

## Controls

### Main Menu
- `↑/↓`: Navigate menu options
- `Enter`: Select option
- `Q`: Quit

### Game Controls
- `H/J/K/L` or `Arrow Keys`: Move (vi-style movement)
- `Y/U/B/N`: Diagonal movement
- `E`: Enter dungeon (when standing on a dungeon entrance 'D')
- `X`: Exit dungeon (when in a dungeon)
- `I`: Open/close inventory
- `Q`: Quit game (or disconnect from multiplayer)

## Terrain Types

- `.` Floor (dungeons)
- `#` Wall (dungeons)
- `"` Grass (overworld)
- `T` Tree (passable but slows movement)
- `^` Mountain (impassable)
- `~` Water (impassable)
- `+` Road (clear path)
- `V` Village (interactive)
- `D` Dungeon Entrance
- `<` Dungeon Exit (inside dungeons, serves as entrance/exit)
- `@` Player (you - yellow)
- `@` Other Players (cyan in multiplayer)

## Architecture

### Client-Server Communication
The game uses WebSocket communication with JSON messages:

**Client Messages:**
- `Connect`: Join the game with a player name
- `Move`: Send movement commands
- `EnterDungeon`/`ExitDungeon`: World transitions
- `OpenInventory`/`CloseInventory`: UI state
- `Disconnect`: Leave the game

**Server Messages:**
- `Connected`: Confirmation with player ID
- `GameState`: Complete world state update
- `PlayerMoved`: Individual player movement
- `PlayerJoined`/`PlayerLeft`: Player management
- `Error`: Error messages
- `Message`: Game events and notifications

### Project Structure
```
src/
├── main.rs       # Client entry point and main game loop
├── server.rs     # Multiplayer server implementation
├── app.rs        # Game state and logic
├── ui.rs         # User interface rendering
├── network.rs    # Client networking
├── protocol.rs   # Shared message types
└── terrain.rs    # World generation
```

## Technical Details

### Dependencies
- `ratatui`: Terminal UI framework
- `tokio`: Async runtime
- `tokio-tungstenite`: WebSocket implementation  
- `serde`/`serde_json`: Serialization
- `uuid`: Unique player IDs
- `url`: URL parsing
- `futures-util`: Async utilities

### Multiplayer Features
- **Efficient Networking**: Only sends updates when game state changes
- **Independent Actions**: Players can open inventory, move, and perform actions independently
- **Shared World**: All players share the same map and can see each other's positions
- **Synchronized Dungeons**: When one player enters/exits a dungeon, all players transition together
- **Real-time Updates**: Player movements and actions are immediately visible to others
- **Graceful Disconnection**: Players can join and leave without affecting others

## Building and Development

### Build All
```bash
cargo build
```

### Build Specific Binary
```bash
cargo build --bin client
cargo build --bin server  
```

### Run in Development
```bash
# Terminal 1: Start server
cargo run --bin server

# Terminal 2: Start client
cargo run --bin client
```

## Testing Multiplayer

1. Start the server: `cargo run --bin server`
2. Open multiple terminals and run the client: `cargo run --bin client`
3. Select "Multiplayer" from the main menu in each client
4. You should see other players as cyan `@` symbols
5. Move around and watch the real-time synchronization
6. Try entering dungeons to see how all players transition together

## Future Improvements

- Player names displayed above characters
- Chat system
- Multiple dungeon instances
- Combat system
- Items and equipment
- Character progression
- Different player classes/abilities
- Configurable server address and port
- Player authentication
- Persistent world state
