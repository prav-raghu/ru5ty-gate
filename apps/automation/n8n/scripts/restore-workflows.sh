#!/bin/bash
# =============================================================================
# n8n Workflow Restore Script
# =============================================================================
# Imports workflows from a backup to an n8n instance
#
# Usage:
#   ./restore-workflows.sh <project-slug> <backup-path> [n8n-url]
#
# Examples:
#   ./restore-workflows.sh acme-corp ./workflows/acme-corp/latest
#   ./restore-workflows.sh acme-corp ./workflows/acme-corp/backup-2026-02-02 https://n8n.acme-corp.yourdomain.co.za
#
# Requirements:
#   - curl
#   - jq
#   - n8n API key (set N8N_API_KEY environment variable)
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Arguments
PROJECT_SLUG="${1:-}"
BACKUP_PATH="${2:-}"
N8N_URL="${3:-http://localhost:4008}"

# Validate arguments
if [[ -z "${PROJECT_SLUG}" ]] || [[ -z "${BACKUP_PATH}" ]]; then
    echo -e "${RED}Error: Project slug and backup path are required${NC}"
    echo "Usage: $0 <project-slug> <backup-path> [n8n-url]"
    exit 1
fi

# Resolve symlinks
if [[ -L "${BACKUP_PATH}" ]]; then
    BACKUP_PATH=$(readlink -f "${BACKUP_PATH}")
fi

# Validate backup directory
if [[ ! -d "${BACKUP_PATH}" ]] || [[ ! -f "${BACKUP_PATH}/manifest.json" ]]; then
    echo -e "${RED}Error: Invalid backup directory. Missing manifest.json${NC}"
    exit 1
fi

# Check for API key
if [[ -z "${N8N_API_KEY:-}" ]]; then
    echo -e "${RED}Error: N8N_API_KEY environment variable is required for restore${NC}"
    exit 1
fi

echo -e "${GREEN}=== n8n Workflow Restore ===${NC}"
echo "Project: ${PROJECT_SLUG}"
echo "Backup: ${BACKUP_PATH}"
echo "Target: ${N8N_URL}"
echo ""

# Read manifest
BACKUP_PROJECT=$(jq -r '.project' "${BACKUP_PATH}/manifest.json")
BACKUP_TIMESTAMP=$(jq -r '.timestamp' "${BACKUP_PATH}/manifest.json")
WORKFLOW_COUNT=$(jq -r '.workflow_count' "${BACKUP_PATH}/manifest.json")

echo -e "${BLUE}Backup Info:${NC}"
echo "  Original Project: ${BACKUP_PROJECT}"
echo "  Timestamp: ${BACKUP_TIMESTAMP}"
echo "  Workflows: ${WORKFLOW_COUNT}"
echo ""

# Confirm restore
read -p "Continue with restore? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Restore cancelled."
    exit 0
fi

# Build auth header
AUTH_HEADER="X-N8N-API-KEY: ${N8N_API_KEY}"

# Function to make API calls
api_post() {
    local endpoint="$1"
    local data="$2"

    curl -s -X POST \
        -H "${AUTH_HEADER}" \
        -H "Content-Type: application/json" \
        -d "${data}" \
        "${N8N_URL}/api/v1/${endpoint}"
}

# Import workflows
echo ""
echo "Importing workflows..."
SUCCESS_COUNT=0
FAIL_COUNT=0

if [[ -d "${BACKUP_PATH}/workflows" ]]; then
    for workflow_file in "${BACKUP_PATH}/workflows"/*.json; do
        if [[ -f "${workflow_file}" ]]; then
            workflow_name=$(jq -r '.name // "unnamed"' "${workflow_file}")
            workflow_id=$(jq -r '.id' "${workflow_file}")

            # Remove id to create new workflow (n8n will assign new id)
            workflow_data=$(jq 'del(.id) | del(.createdAt) | del(.updatedAt)' "${workflow_file}")

            echo -n "  Importing: ${workflow_name}... "

            result=$(api_post "workflows" "${workflow_data}" 2>&1)

            if echo "${result}" | jq -e '.id' > /dev/null 2>&1; then
                new_id=$(echo "${result}" | jq -r '.id')
                echo -e "${GREEN}✓${NC} (new id: ${new_id})"
                ((SUCCESS_COUNT++))
            else
                echo -e "${RED}✗${NC}"
                error=$(echo "${result}" | jq -r '.message // "Unknown error"' 2>/dev/null || echo "${result}")
                echo -e "    ${RED}Error: ${error}${NC}"
                ((FAIL_COUNT++))
            fi
        fi
    done
fi

# Import tags (if available)
if [[ -f "${BACKUP_PATH}/tags.json" ]]; then
    echo ""
    echo "Importing tags..."
    TAG_COUNT=$(jq '.data | length' "${BACKUP_PATH}/tags.json" 2>/dev/null || echo "0")

    if [[ "${TAG_COUNT}" -gt 0 ]]; then
        jq -c '.data[]' "${BACKUP_PATH}/tags.json" 2>/dev/null | while read -r tag; do
            tag_name=$(echo "${tag}" | jq -r '.name')
            tag_data=$(echo "${tag}" | jq 'del(.id)')

            echo -n "  Importing tag: ${tag_name}... "
            result=$(api_post "tags" "${tag_data}" 2>&1)

            if echo "${result}" | jq -e '.id' > /dev/null 2>&1; then
                echo -e "${GREEN}✓${NC}"
            else
                echo -e "${YELLOW}skipped (may already exist)${NC}"
            fi
        done
    fi
fi

echo ""
echo -e "${GREEN}=== Restore Complete ===${NC}"
echo "Successful: ${SUCCESS_COUNT}"
echo "Failed: ${FAIL_COUNT}"
echo ""

if [[ ${FAIL_COUNT} -gt 0 ]]; then
    echo -e "${YELLOW}Some workflows failed to import. Check errors above.${NC}"
fi

echo -e "${YELLOW}Note: Credentials need to be recreated manually.${NC}"
echo "The original credentials are encrypted and cannot be transferred."
