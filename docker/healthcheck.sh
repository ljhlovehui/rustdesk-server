#!/bin/bash

# RustDesk Enterprise Server Health Check Script

# Default ports
HBBS_PORT=${HBBS_PORT:-21115}
WEB_PORT=${WEB_PORT:-21119}

# Function to check if port is listening
check_port() {
    local port=$1
    local service=$2
    
    if nc -z localhost "$port" 2>/dev/null; then
        echo "✓ $service (port $port) is healthy"
        return 0
    else
        echo "✗ $service (port $port) is not responding"
        return 1
    fi
}

# Function to check web API
check_web_api() {
    local port=$1
    
    # Try to get health endpoint
    if curl -f -s "http://localhost:$port/api/health" >/dev/null 2>&1; then
        echo "✓ Web API (port $port) is healthy"
        return 0
    else
        echo "✗ Web API (port $port) is not responding"
        return 1
    fi
}

# Function to check database
check_database() {
    local db_url=${DATABASE_URL:-"sqlite:///app/data/enterprise.sqlite3"}
    
    if [[ "$db_url" == sqlite://* ]]; then
        local db_file=${db_url#sqlite://}
        if [ -f "$db_file" ] && [ -r "$db_file" ]; then
            echo "✓ Database file is accessible"
            return 0
        else
            echo "✗ Database file is not accessible: $db_file"
            return 1
        fi
    else
        echo "✓ Database URL configured (external database)"
        return 0
    fi
}

# Main health check
main() {
    echo "RustDesk Enterprise Server Health Check"
    echo "======================================="
    
    local exit_code=0
    
    # Check HBBS service
    if ! check_port "$HBBS_PORT" "HBBS Service"; then
        exit_code=1
    fi
    
    # Check Web interface (if this is HBBS container)
    if [ "$1" != "hbbr" ]; then
        if ! check_web_api "$WEB_PORT"; then
            exit_code=1
        fi
        
        # Check database
        if ! check_database; then
            exit_code=1
        fi
    fi
    
    # Check system resources
    echo ""
    echo "System Resources:"
    echo "=================="
    
    # Memory usage
    local mem_usage=$(free | grep Mem | awk '{printf "%.1f", $3/$2 * 100.0}')
    echo "Memory usage: ${mem_usage}%"
    
    # Disk usage for data directory
    local disk_usage=$(df /app/data 2>/dev/null | tail -1 | awk '{print $5}' | sed 's/%//')
    if [ -n "$disk_usage" ]; then
        echo "Disk usage (/app/data): ${disk_usage}%"
        
        # Warn if disk usage is high
        if [ "$disk_usage" -gt 90 ]; then
            echo "⚠ Warning: High disk usage!"
            exit_code=1
        fi
    fi
    
    # Check if log directory is writable
    if [ -w "/app/logs" ]; then
        echo "✓ Log directory is writable"
    else
        echo "✗ Log directory is not writable"
        exit_code=1
    fi
    
    echo ""
    if [ $exit_code -eq 0 ]; then
        echo "✓ All health checks passed"
    else
        echo "✗ Some health checks failed"
    fi
    
    exit $exit_code
}

# Install netcat if not available
if ! command -v nc >/dev/null 2>&1; then
    echo "Installing netcat for port checking..."
    apt-get update >/dev/null 2>&1 && apt-get install -y netcat-openbsd >/dev/null 2>&1
fi

# Install curl if not available
if ! command -v curl >/dev/null 2>&1; then
    echo "Installing curl for API checking..."
    apt-get update >/dev/null 2>&1 && apt-get install -y curl >/dev/null 2>&1
fi

# Run main function
main "$@"