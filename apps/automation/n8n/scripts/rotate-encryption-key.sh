#!/bin/bash
# =============================================================================
# n8n Encryption Key Rotation Script
# =============================================================================
# Rotates the N8N_ENCRYPTION_KEY for a project instance
#
# WARNING: This is a destructive operation. All credentials will need to be
# re-entered after key rotation.
#
# Usage:
#   ./rotate-encryption-key.sh <project-slug>
#
# Requirements:
#   - openssl
#   - Access to modify environment variables or secrets
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Arguments
PROJECT_SLUG="${1:-}"

if [[ -z "${PROJECT_SLUG}" ]]; then
    echo -e "${RED}Error: Project slug is required${NC}"
    echo "Usage: $0 <project-slug>"
    exit 1
fi

echo -e "${RED}==============================================================================${NC}"
echo -e "${RED}                    WARNING: ENCRYPTION KEY ROTATION                          ${NC}"
echo -e "${RED}==============================================================================${NC}"
echo ""
echo "This will generate a new encryption key for project: ${PROJECT_SLUG}"
echo ""
echo -e "${YELLOW}IMPORTANT:${NC}"
echo "  1. All stored credentials will become UNREADABLE"
echo "  2. You must re-enter ALL credential values in n8n after rotation"
echo "  3. This cannot be undone without the old key"
echo ""
echo -e "${YELLOW}Before proceeding, ensure you have:${NC}"
echo "  - Documented all credential values"
echo "  - Created a backup of the database"
echo "  - Stopped the n8n instance"
echo ""

read -p "Do you understand the risks and want to proceed? (type 'yes' to confirm): " -r
echo

if [[ "${REPLY}" != "yes" ]]; then
    echo "Key rotation cancelled."
    exit 0
fi

# Generate new key
NEW_KEY=$(openssl rand -hex 32)
NEW_KEY_BASE64=$(echo -n "${NEW_KEY}" | base64)

echo ""
echo -e "${GREEN}New encryption key generated:${NC}"
echo ""
echo "Plain text (for .env files):"
echo -e "${YELLOW}N8N_ENCRYPTION_KEY=${NEW_KEY}${NC}"
echo ""
echo "Base64 encoded (for Kubernetes secrets):"
echo -e "${YELLOW}${NEW_KEY_BASE64}${NC}"
echo ""

# Update instructions
echo -e "${GREEN}=== Next Steps ===${NC}"
echo ""
echo "1. For Docker Compose:"
echo "   - Edit: apps/automation/n8n/compose/.env.${PROJECT_SLUG}"
echo "   - Update: N8N_ENCRYPTION_KEY=${NEW_KEY}"
echo ""
echo "2. For Kubernetes:"
echo "   - Update the secret with base64 value above"
echo "   - Apply: kubectl apply -f secret.yaml -n ${PROJECT_SLUG}"
echo ""
echo "3. Restart n8n instance"
echo ""
echo "4. Re-enter all credential values in n8n UI"
echo ""

# Optionally save to file
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEY_FILE="${SCRIPT_DIR}/../.keys/${PROJECT_SLUG}-$(date +%Y%m%d%H%M%S).key"
mkdir -p "$(dirname "${KEY_FILE}")"

read -p "Save new key to file? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "N8N_ENCRYPTION_KEY=${NEW_KEY}" > "${KEY_FILE}"
    chmod 600 "${KEY_FILE}"
    echo -e "${GREEN}Key saved to: ${KEY_FILE}${NC}"
    echo -e "${RED}WARNING: Add .keys/ to .gitignore if not already present!${NC}"
fi
