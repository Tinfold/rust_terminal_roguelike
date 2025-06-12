#!/bin/bash

# Multiplayer Roguelike Launch Script

echo "🗡️  MULTIPLAYER ROGUELIKE LAUNCHER  🛡️"
echo ""
echo "Choose an option:"
echo "1) Start Server"
echo "2) Start Client"  
echo "3) Start Server + Client (in new terminal)"
echo "4) Build Project"
echo "5) Exit"
echo ""

read -p "Enter your choice (1-5): " choice

case $choice in
    1)
        echo "Starting server on 127.0.0.1:8080..."
        cargo run --bin server
        ;;
    2)
        echo "Starting client..."
        cargo run --bin client
        ;;
    3)
        echo "Starting server in background..."
        cargo run --bin server &
        SERVER_PID=$!
        sleep 2
        echo "Starting client..."
        if command -v gnome-terminal &> /dev/null; then
            gnome-terminal -- bash -c "cd '$(pwd)' && cargo run --bin client; read -p 'Press Enter to close...'"
        elif command -v xterm &> /dev/null; then
            xterm -e bash -c "cd '$(pwd)' && cargo run --bin client; read -p 'Press Enter to close...'" &
        else
            echo "Opening client in new terminal not supported. Please run 'cargo run --bin client' in another terminal."
        fi
        echo "Server PID: $SERVER_PID (use 'kill $SERVER_PID' to stop)"
        wait $SERVER_PID
        ;;
    4)
        echo "Building project..."
        cargo build
        echo "Build complete!"
        ;;
    5)
        echo "Goodbye! 👋"
        exit 0
        ;;
    *)
        echo "Invalid choice. Please run the script again."
        exit 1
        ;;
esac
