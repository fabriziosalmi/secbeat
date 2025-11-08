#!/bin/bash
# SecBeat Mitigation Node Docker Entrypoint
# Handles TLS certificate generation and service startup

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}SecBeat Mitigation Node - Docker Entrypoint${NC}"
echo "==========================================="

# Function to generate self-signed certificates
generate_self_signed_certs() {
    local cert_path="$1"
    local key_path="$2"
    
    echo -e "${YELLOW}Generating self-signed TLS certificates...${NC}"
    
    # Create directory if it doesn't exist
    mkdir -p "$(dirname "$cert_path")"
    mkdir -p "$(dirname "$key_path")"
    
    # Generate self-signed certificate
    openssl req -x509 -newkey rsa:4096 \
        -keyout "$key_path" \
        -out "$cert_path" \
        -days 365 -nodes \
        -subj "/C=US/ST=State/L=City/O=SecBeat/CN=${SECBEAT_HOSTNAME:-localhost}" \
        2>/dev/null
    
    # Set proper permissions
    chmod 644 "$cert_path"
    chmod 600 "$key_path"
    
    echo -e "${GREEN}✓ Self-signed certificates generated${NC}"
    echo "  Certificate: $cert_path"
    echo "  Private Key: $key_path"
}

# Function to check if certificates exist and are valid
check_certificates() {
    local cert_path="$1"
    local key_path="$2"
    
    # Check if files exist
    if [ ! -f "$cert_path" ] || [ ! -f "$key_path" ]; then
        return 1
    fi
    
    # Check if certificate file is not empty
    if [ ! -s "$cert_path" ] || [ ! -s "$key_path" ]; then
        echo -e "${YELLOW}Warning: Certificate files exist but are empty${NC}"
        return 1
    fi
    
    # Check if certificate is valid (not expired)
    if ! openssl x509 -in "$cert_path" -noout -checkend 86400 2>/dev/null; then
        echo -e "${YELLOW}Warning: Certificate will expire within 24 hours${NC}"
    fi
    
    return 0
}

# Main certificate handling logic
handle_certificates() {
    local config_file="${SECBEAT_CONFIG:-config.dev}.toml"
    
    # Try to extract cert paths from config file
    local cert_path=$(grep -E '^\s*cert_path\s*=' "$config_file" 2>/dev/null | sed -E 's/.*=\s*"([^"]+)".*/\1/' || echo "certs/cert.pem")
    local key_path=$(grep -E '^\s*key_path\s*=' "$config_file" 2>/dev/null | sed -E 's/.*=\s*"([^"]+)".*/\1/' || echo "certs/key.pem")
    
    echo "Checking TLS certificates..."
    echo "  Config file: $config_file"
    echo "  Expected cert: $cert_path"
    echo "  Expected key: $key_path"
    
    if check_certificates "$cert_path" "$key_path"; then
        echo -e "${GREEN}✓ Valid TLS certificates found${NC}"
        
        # Display certificate information
        echo ""
        echo "Certificate Information:"
        openssl x509 -in "$cert_path" -noout -subject -issuer -dates 2>/dev/null | sed 's/^/  /'
    else
        echo -e "${YELLOW}⚠ Valid certificates not found${NC}"
        
        # Check if we should auto-generate
        if [ "${SECBEAT_AUTO_GENERATE_CERTS:-true}" = "true" ]; then
            generate_self_signed_certs "$cert_path" "$key_path"
        else
            echo -e "${RED}Error: TLS certificates required but not found${NC}"
            echo "Please mount certificates as volumes or set SECBEAT_AUTO_GENERATE_CERTS=true"
            exit 1
        fi
    fi
}

# Handle TLS certificates if TLS is enabled
if [ "${SECBEAT_TLS_ENABLED:-true}" = "true" ]; then
    handle_certificates
else
    echo -e "${YELLOW}TLS is disabled - skipping certificate check${NC}"
fi

echo ""
echo "Starting mitigation-node..."
echo "  Config: ${SECBEAT_CONFIG:-config.dev}"
echo "  Log level: ${RUST_LOG:-info}"
echo ""

# Execute the main command
exec "$@"
