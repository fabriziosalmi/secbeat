#!/bin/bash

# Test Proxmox API authentication
echo "Testing Proxmox API authentication..."

PROXMOX_HOST="192.168.100.23"
PROXMOX_USER="root"

# Prompt for password
echo -n "Enter Proxmox password for user '$PROXMOX_USER': "
read -s PROXMOX_PASSWORD
echo

# Test API connection
echo "Testing API connection to https://${PROXMOX_HOST}:8006/api2/json/version"

curl -k -d "username=${PROXMOX_USER}@pam&password=${PROXMOX_PASSWORD}" \
     "https://${PROXMOX_HOST}:8006/api2/json/access/ticket" \
     2>/dev/null | python3 -m json.tool

if [ $? -eq 0 ]; then
    echo "✓ Authentication successful"
else
    echo "✗ Authentication failed"
fi

# Test node listing
echo -e "\nTesting node listing..."
TICKET_DATA=$(curl -k -s -d "username=${PROXMOX_USER}@pam&password=${PROXMOX_PASSWORD}" \
              "https://${PROXMOX_HOST}:8006/api2/json/access/ticket")

if [ $? -eq 0 ]; then
    TICKET=$(echo "$TICKET_DATA" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['ticket'])" 2>/dev/null)
    CSRF_TOKEN=$(echo "$TICKET_DATA" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['CSRFPreventionToken'])" 2>/dev/null)
    
    if [ -n "$TICKET" ] && [ -n "$CSRF_TOKEN" ]; then
        echo "Got authentication ticket, testing node list..."
        curl -k -H "Cookie: PVEAuthCookie=$TICKET" \
             -H "CSRFPreventionToken: $CSRF_TOKEN" \
             "https://${PROXMOX_HOST}:8006/api2/json/nodes" \
             2>/dev/null | python3 -m json.tool
    else
        echo "Could not extract authentication tokens"
    fi
fi
