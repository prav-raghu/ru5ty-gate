#!/bin/bash
# =============================================================================
# n8n Workflow Backup Script
# =============================================================================
# Exports all workflows and credentials metadata from an n8n instance
#
# Usage:
#   ./backup-workflows.sh <project-slug> [n8n-url]
#
# Examples:
#   ./backup-workflows.sh acme-corp
#   ./backup-workflows.sh acme-corp https://n8n.acme-corp.yourdomain.co.za
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
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKFLOWS_DIR="${SCRIPT_DIR}/../workflows"

# Arguments
PROJECT_SLUG="${1:-}"
N8N_URL="${2:-http://localhost:4008}"

# Validate arguments
if [[ -z "${PROJECT_SLUG}" ]]; then
    echo -e "${RED}Error: Project slug is required${NC}"
    echo "Usage: $0 <project-slug> [n8n-url]"
    exit 1
fi

# Check for API key
if [[ -z "${N8N_API_KEY:-}" ]]; then
    echo -e "${YELLOW}Warning: N8N_API_KEY not set. Using basic auth if configured.${NC}"
fi

# Create backup directory
TIMESTAMP=$(date +"%Y-%m-%d_%H-%M-%S")
BACKUP_DIR="${WORKFLOWS_DIR}/${PROJECT_SLUG}/backup-${TIMESTAMP}"
mkdir -p "${BACKUP_DIR}"

echo -e "${GREEN}=== n8n Workflow Backup ===${NC}"
echo "Project: ${PROJECT_SLUG}"
echo "URL: ${N8N_URL}"
echo "Backup directory: ${BACKUP_DIR}"
echo ""

# Build auth header
AUTH_HEADER=""
if [[ -n "${N8N_API_KEY:-}" ]]; then
    AUTH_HEADER="X-N8N-API-KEY: ${N8N_API_KEY}"
elif [[ -n "${N8N_BASIC_AUTH_USER:-}" ]] && [[ -n "${N8N_BASIC_AUTH_PASSWORD:-}" ]]; then
    AUTH_HEADER="Authorization: Basic $(echo -n "${N8N_BASIC_AUTH_USER}:${N8N_BASIC_AUTH_PASSWORD}" | base64)"
fi

# Function to make API calls
api_call() {
    local endpoint="$1"
    local output_file="$2"

    if [[ -n "${AUTH_HEADER}" ]]; then
        curl -s -H "${AUTH_HEADER}" "${N8N_URL}/api/v1/${endpoint}" -o "${output_file}"
    else
        curl -s "${N8N_URL}/api/v1/${endpoint}" -o "${output_file}"
    fi
}

# Export workflows
echo "Exporting workflows..."
api_call "workflows" "${BACKUP_DIR}/workflows.json"

# Check if export was successful
if [[ ! -s "${BACKUP_DIR}/workflows.json" ]]; then
    echo -e "${RED}Error: Failed to export workflows. Check your API key and URL.${NC}"
    exit 1
fi

# Count workflows
WORKFLOW_COUNT=$(jq '.data | length' "${BACKUP_DIR}/workflows.json" 2>/dev/null || echo "0")
echo -e "${GREEN}✓ Exported ${WORKFLOW_COUNT} workflows${NC}"

# Export individual workflows (with full details)
mkdir -p "${BACKUP_DIR}/workflows"
if [[ "${WORKFLOW_COUNT}" -gt 0 ]]; then
    echo "Exporting individual workflow details..."
    jq -r '.data[].id' "${BACKUP_DIR}/workflows.json" 2>/dev/null | while read -r workflow_id; do
        if [[ -n "${workflow_id}" ]]; then
            api_call "workflows/${workflow_id}" "${BACKUP_DIR}/workflows/${workflow_id}.json"
            workflow_name=$(jq -r '.name // "unnamed"' "${BACKUP_DIR}/workflows/${workflow_id}.json" 2>/dev/null)
            echo "  - ${workflow_name} (${workflow_id})"
        fi
    done
fi

# Export credentials (metadata only - actual secrets are encrypted)
echo ""
echo "Exporting credentials metadata..."
api_call "credentials" "${BACKUP_DIR}/credentials.json"
CREDENTIAL_COUNT=$(jq '.data | length' "${BACKUP_DIR}/credentials.json" 2>/dev/null || echo "0")
echo -e "${GREEN}✓ Exported ${CREDENTIAL_COUNT} credentials metadata${NC}"

# Export tags (if available)
echo ""
echo "Exporting tags..."
api_call "tags" "${BACKUP_DIR}/tags.json" 2>/dev/null || echo "{}" > "${BACKUP_DIR}/tags.json"

# Create backup manifest
cat > "${BACKUP_DIR}/manifest.json" << EOF
{
  "project": "${PROJECT_SLUG}",
  "timestamp": "${TIMESTAMP}",
  "n8n_url": "${N8N_URL}",
  "workflow_count": ${WORKFLOW_COUNT},
  "credential_count": ${CREDENTIAL_COUNT},
  "backup_version": "1.0"
}
EOF

# Create a latest symlink
LATEST_LINK="${WORKFLOWS_DIR}/${PROJECT_SLUG}/latest"
rm -f "${LATEST_LINK}"
ln -sf "backup-${TIMESTAMP}" "${LATEST_LINK}"

echo ""
echo -e "${GREEN}=== Backup Complete ===${NC}"
echo "Location: ${BACKUP_DIR}"
echo "Manifest: ${BACKUP_DIR}/manifest.json"
echo ""
echo -e "${YELLOW}Note: Credential secrets are not exported (encrypted in database).${NC}"
echo "You'll need to re-enter credential values when restoring to a new instance."
