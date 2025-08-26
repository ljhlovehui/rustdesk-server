#!/bin/bash
set -e

# RustDesk Enterprise Server Entrypoint Script

# Default values
HBBS_PORT=${HBBS_PORT:-21115}
HBBR_PORT=${HBBR_PORT:-21117}
WEB_PORT=${WEB_PORT:-21119}
RUSTDESK_KEY=${RUSTDESK_KEY:-""}
JWT_SECRET=${JWT_SECRET:-""}
DATABASE_URL=${DATABASE_URL:-"sqlite:///app/data/enterprise.sqlite3"}

# Create data directory if it doesn't exist
mkdir -p /app/data /app/logs

# Function to generate random key
generate_key() {
    openssl rand -base64 32 | tr -d "=+/" | cut -c1-25
}

# Generate keys if not provided
if [ -z "$RUSTDESK_KEY" ]; then
    RUSTDESK_KEY=$(generate_key)
    echo "Generated RUSTDESK_KEY: $RUSTDESK_KEY"
    echo "Please save this key for client configuration!"
fi

if [ -z "$JWT_SECRET" ]; then
    JWT_SECRET=$(generate_key)$(generate_key)
    echo "Generated JWT_SECRET (saved internally)"
fi

# Export environment variables
export RUSTDESK_KEY
export JWT_SECRET
export DATABASE_URL
export ENTERPRISE_DB_URL="$DATABASE_URL"

# Determine which service to start
SERVICE=${1:-"hbbs"}

case "$SERVICE" in
    "hbbs"|"hbbs-enterprise")
        echo "Starting RustDesk Enterprise HBBS Server..."
        echo "Port: $HBBS_PORT"
        echo "Web Interface: $WEB_PORT"
        echo "Key: $RUSTDESK_KEY"
        
        exec /app/bin/hbbs-enterprise \
            --enterprise \
            --port "$HBBS_PORT" \
            --web-port "$WEB_PORT" \
            --key "$RUSTDESK_KEY" \
            --db-url "$DATABASE_URL"
        ;;
    
    "hbbr"|"hbbr-enterprise")
        echo "Starting RustDesk Enterprise HBBR Relay Server..."
        echo "Port: $HBBR_PORT"
        echo "Key: $RUSTDESK_KEY"
        
        exec /app/bin/hbbr-enterprise \
            --port "$HBBR_PORT" \
            --key "$RUSTDESK_KEY"
        ;;
    
    "utils"|"rustdesk-utils-enterprise")
        echo "Running RustDesk Enterprise Utils..."
        shift
        exec /app/bin/rustdesk-utils-enterprise "$@"
        ;;
    
    *)
        echo "Usage: $0 {hbbs|hbbr|utils} [options]"
        echo ""
        echo "Services:"
        echo "  hbbs    - Start HBBS server (default)"
        echo "  hbbr    - Start HBBR relay server"
        echo "  utils   - Run utilities"
        echo ""
        echo "Environment Variables:"
        echo "  HBBS_PORT      - HBBS server port (default: 21115)"
        echo "  HBBR_PORT      - HBBR relay port (default: 21117)"
        echo "  WEB_PORT       - Web interface port (default: 21119)"
        echo "  RUSTDESK_KEY   - Server key (auto-generated if not set)"
        echo "  JWT_SECRET     - JWT secret (auto-generated if not set)"
        echo "  DATABASE_URL   - Database URL (default: sqlite:///app/data/enterprise.sqlite3)"
        exit 1
        ;;
esac