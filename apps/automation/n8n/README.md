# n8n Workflow Automation

This directory contains the infrastructure configuration for **n8n** workflow automation instances. Each project/client gets its own isolated n8n instance with separate database, encryption keys, and storage.

## Architecture Overview

n8n is deployed as an independent service (not an npm package) following the same pattern as Strapi CMS. Each instance is fully isolated per project to ensure:

- **Data Segregation**: Separate databases and encryption keys
- **Security Isolation**: Independent credentials and access controls
- **Scalability**: Per-project resource allocation
- **Portability**: Easy migration and backup/restore per project

## Directory Structure

```
apps/automation/n8n/
├── compose/
│   ├── docker-compose.template.yml    # Base template for new instances
│   ├── docker-compose.dev.yml         # Development instance
│   ├── .env.example                   # Environment variables template
│   └── .env.dev                       # Dev environment (git-ignored)
├── local/
│   ├── .env.local.example             # Local (non-Docker) env template
│   └── .env.local                     # Local env (git-ignored)
├── k8s/
│   ├── namespace.yaml                 # Namespace template per project
│   ├── deployment.yaml                # n8n deployment manifest
│   ├── service.yaml                   # ClusterIP service
│   ├── ingress.yaml                   # Ingress with TLS
│   ├── secret.template.yaml           # Secrets template
│   ├── configmap.yaml                 # Configuration
│   └── pvc.yaml                       # Persistent volume claim
├── scripts/
│   ├── backup-workflows.sh            # Export workflows to JSON
│   ├── restore-workflows.sh           # Import workflows from JSON
│   ├── create-instance.ts             # Create new project instance
│   ├── start-local.ts                 # Run n8n locally without Docker
│   └── rotate-encryption-key.sh       # Rotate encryption key
├── workflows/
│   └── .gitkeep                       # Exported workflows per project
└── README.md
```

## Quick Start (Development)

### Option A: Using Docker (Recommended)

#### Prerequisites

- Docker and Docker Compose installed
- PostgreSQL database available (can use the monorepo's shared dev instance)

#### 1. Start Development Instance

```bash
cd apps/automation/n8n/compose

# Copy and configure environment
cp .env.example .env.dev

# Edit .env.dev with your settings
# IMPORTANT: Generate a unique N8N_ENCRYPTION_KEY

# Start n8n
docker compose -f docker-compose.dev.yml up -d
```

#### 2. Access n8n

- **URL**: http://localhost:4008
- **Default credentials**: Set on first access

---

### Option B: Running Locally (Without Docker)

Run n8n directly on your machine using Node.js. This is useful for debugging or when Docker isn't available.

#### Prerequisites

- Node.js 18+ installed
- PostgreSQL running (locally or via Docker)
- n8n installed globally

#### 1. Install n8n Globally

```bash
npm install -g n8n
```

#### 2. Ensure PostgreSQL is Running

If using the monorepo's Docker PostgreSQL:

```bash
# Start only PostgreSQL from the dev stack
docker compose -f devops/docker-compose.dev.yml up -d postgres

# Create the n8n database
docker exec -it ru5ty-gate-postgres-dev psql -U postgres -c "CREATE DATABASE n8n_dev;"
```

Or connect to your local PostgreSQL installation.

#### 3. Configure Environment

```bash
cd apps/automation/n8n/local

# Copy the example environment file
cp .env.local.example .env.local

# Edit .env.local with your database credentials
```

#### 4. Start n8n Locally

**Option 1: Using the start script**

```bash
# From monorepo root
npx ts-node apps/automation/n8n/scripts/start-local.ts

# Or using pnpm script (if configured)
pnpm n8n:local
```

**Option 2: Direct command with environment variables**

```bash
cd apps/automation/n8n/local

# Load environment and start n8n
# PowerShell:
Get-Content .env.local | ForEach-Object { if ($_ -match '^([^#][^=]*)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2]) } }; n8n start

# Bash/Linux/Mac:
export $(grep -v '^#' .env.local | xargs) && n8n start
```

**Option 3: Minimal quick start (SQLite - no PostgreSQL needed)**

```bash
# For quick testing without any database setup
n8n start
```

This uses SQLite by default and stores data in `~/.n8n/`.

#### 5. Access n8n

- **URL**: http://localhost:4008
- **Data folder**: `apps/automation/n8n/local/.n8n/` (when using start script)

#### Local Development Tips

| Scenario | Command |
|----------|---------|
| Start with verbose logging | `N8N_LOG_LEVEL=debug n8n start` |
| Use different port | `N8N_PORT=5679 n8n start` |
| Export workflows | `n8n export:workflow --all --output=./backups/` |
| Import workflows | `n8n import:workflow --input=./backups/workflow.json` |
| Execute workflow by ID | `n8n execute --id=<workflow-id>` |

## Project Instance Conventions

### Naming Standards

| Resource | Pattern | Example |
|----------|---------|---------|
| Service/Container | `n8n-<projectSlug>` | `n8n-acme-corp` |
| Database | `n8n_<projectSlug>` | `n8n_acme_corp` |
| Docker Volume | `n8n_data_<projectSlug>` | `n8n_data_acme_corp` |
| K8s Namespace | `<projectSlug>` | `acme-corp` |
| URL | `https://n8n.<projectSlug>.yourdomain.co.za` | `https://n8n.acme-corp.yourdomain.co.za` |

### Project Slug Rules

- Lowercase letters, numbers, and hyphens only
- Must start with a letter
- Maximum 63 characters (DNS label limit)
- Examples: `acme-corp`, `client-xyz`, `project-alpha`

## Environment Variables

### Required Variables (Per Instance)

| Variable | Description | Example |
|----------|-------------|---------|
| `N8N_ENCRYPTION_KEY` | **CRITICAL**: Unique 32+ char key for encrypting credentials | `openssl rand -hex 32` |
| `N8N_HOST` | Hostname for n8n instance | `n8n.acme-corp.yourdomain.co.za` |
| `N8N_PROTOCOL` | HTTP or HTTPS | `https` |
| `N8N_PORT` | Port n8n listens on | `5678` |
| `WEBHOOK_URL` | External webhook URL | `https://n8n.acme-corp.yourdomain.co.za` |
| `DB_POSTGRESDB_HOST` | PostgreSQL host | `postgres` |
| `DB_POSTGRESDB_DATABASE` | Database name | `n8n_acme_corp` |
| `DB_POSTGRESDB_USER` | Database user | `n8n_acme_corp` |
| `DB_POSTGRESDB_PASSWORD` | Database password | `secure-password` |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `N8N_BASIC_AUTH_ACTIVE` | Enable basic auth | `true` |
| `N8N_BASIC_AUTH_USER` | Basic auth username | - |
| `N8N_BASIC_AUTH_PASSWORD` | Basic auth password | - |
| `N8N_LOG_LEVEL` | Logging level | `info` |
| `N8N_METRICS` | Enable Prometheus metrics | `false` |
| `EXECUTIONS_DATA_PRUNE` | Auto-prune old executions | `true` |
| `EXECUTIONS_DATA_MAX_AGE` | Max age of execution data (hours) | `168` |

## Creating a New Project Instance

### Using the Script (Recommended)

```bash
# From monorepo root
npx ts-node apps/automation/n8n/scripts/create-instance.ts \
  --project acme-corp \
  --domain yourdomain.co.za \
  --db-host postgres \
  --db-password "secure-password"
```

This generates:
- `compose/docker-compose.acme-corp.yml`
- `compose/.env.acme-corp`
- `k8s/acme-corp/` directory with all manifests

### Manual Setup

1. Copy `compose/docker-compose.template.yml` to `docker-compose.<projectSlug>.yml`
2. Copy `compose/.env.example` to `.env.<projectSlug>`
3. Update all `<PROJECT_SLUG>` placeholders
4. Generate unique `N8N_ENCRYPTION_KEY`
5. Create database and user
6. Start the instance

## Docker Compose Deployment

### Single Instance

```bash
cd apps/automation/n8n/compose
docker compose -f docker-compose.acme-corp.yml up -d
```

### Multiple Instances

```bash
# Start multiple project instances
docker compose -f docker-compose.acme-corp.yml up -d
docker compose -f docker-compose.client-xyz.yml up -d
```

## Kubernetes Deployment

### Per-Namespace Pattern (Recommended)

Each project gets its own Kubernetes namespace with isolated resources.

```bash
# Create namespace and deploy
kubectl apply -f k8s/acme-corp/namespace.yaml
kubectl apply -f k8s/acme-corp/secret.yaml
kubectl apply -f k8s/acme-corp/configmap.yaml
kubectl apply -f k8s/acme-corp/pvc.yaml
kubectl apply -f k8s/acme-corp/deployment.yaml
kubectl apply -f k8s/acme-corp/service.yaml
kubectl apply -f k8s/acme-corp/ingress.yaml
```

### Using Kustomize

```bash
kubectl apply -k k8s/acme-corp/
```

## Integration with Monorepo Services

### n8n Calling Your APIs (Recommended)

n8n acts as the orchestrator, calling your backend services:

```
[n8n Workflow]
    │
    ├──► POST https://api.yourdomain.co.za/v1/customers
    ├──► GET  https://api.yourdomain.co.za/v1/orders
    └──► POST https://api.yourdomain.co.za/v1/notifications
```

Configure in n8n:
1. Create HTTP Request nodes
2. Use API Gateway endpoints
3. Store API keys as n8n credentials

### Your APIs Triggering n8n (Webhooks)

Backend services trigger n8n workflows via webhooks:

```typescript
// In your customer-api service
async function triggerWorkflow(event: string, data: unknown): Promise<void> {
  await axios.post(`${N8N_WEBHOOK_URL}/webhook/${event}`, data, {
    headers: { 'X-N8N-API-KEY': process.env.N8N_API_KEY }
  });
}
```

### Event Bus Integration (Advanced)

For complex scenarios, use Redis/RabbitMQ:

```
[Backend Service] ──► [Redis/RabbitMQ] ◄── [n8n Trigger Node]
```

## Workflow Management

### Backup Workflows

```bash
# Backup all workflows for a project
./scripts/backup-workflows.sh acme-corp

# Output: workflows/acme-corp/backup-2026-02-02/
```

### Restore Workflows

```bash
# Restore workflows to a new instance
./scripts/restore-workflows.sh acme-corp ./workflows/acme-corp/backup-2026-02-02
```

### Version Control Strategy

1. **Runtime source of truth**: Database (n8n manages state)
2. **Backup/audit**: Periodic exports to `workflows/<projectSlug>/`
3. **Migration**: Use backup/restore for environment promotion

## Security Best Practices

### Encryption Key Management

- **CRITICAL**: Never share `N8N_ENCRYPTION_KEY` between projects
- Store encryption keys in secure vault (AWS Secrets Manager, HashiCorp Vault)
- Rotate keys periodically using the provided script

### Network Security

- Place behind reverse proxy (nginx/traefik)
- Enable HTTPS with valid certificates
- Use network policies in Kubernetes
- Restrict webhook access with API keys

### Authentication

- Enable basic auth or SSO
- Use strong passwords
- Consider OAuth2/OIDC integration for enterprise

## Monitoring & Logging

### Prometheus Metrics

Enable metrics endpoint:

```yaml
environment:
  - N8N_METRICS=true
  - N8N_METRICS_PREFIX=n8n_
```

Scrape endpoint: `http://n8n:5678/metrics`

### Logging

Configure log output:

```yaml
environment:
  - N8N_LOG_LEVEL=info
  - N8N_LOG_OUTPUT=console
```

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| Credentials not decrypting | Check `N8N_ENCRYPTION_KEY` matches original |
| Webhooks not receiving | Verify `WEBHOOK_URL` is accessible externally |
| Database connection failed | Check DB credentials and network access |
| Slow executions | Increase container resources, enable Redis queue |

### Debug Mode

```yaml
environment:
  - N8N_LOG_LEVEL=debug
```

### Health Check

```bash
curl http://localhost:4008/healthz
```

## Related Documentation

- [n8n Official Docs](https://docs.n8n.io/)
- [n8n Docker Setup](https://docs.n8n.io/hosting/installation/docker/)
- [n8n Kubernetes](https://docs.n8n.io/hosting/installation/kubernetes/)
- [Monorepo Architecture](../../README.md)
