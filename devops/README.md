# DevOps Documentation for Project

This directory contains the necessary configurations and scripts for deploying and managing the application using Docker and Kubernetes.

## Directory Structure

- **docker/**: Contains Dockerfiles for building images for the API gateway, customer API, and admin API.
- **docker-compose.yml**: Defines the services and configurations for running the application locally using Docker Compose.
- **docker-compose.dev.yml**: Development-focused compose file with additional tooling (Adminer, Redis Commander, OpenObserve).
- **k8s/**: Contains Kubernetes deployment configurations for the API gateway, customer API, admin API, and Redis.
- **scripts/**: Includes various scripts for managing the development environment, running tests, and performing database migrations.

## Related Components

- **apps/automation/n8n/**: Workflow automation platform (n8n) with per-project isolation. See [n8n README](../apps/automation/n8n/README.md) for detailed setup instructions.

## Getting Started

To get started with the project, follow these steps:

1. **Clone the Repository**:
   ```
   git clone <repository-url>
   cd ru5ty-gate
   ```

2. **Build Docker Images**:
   You can build the Docker images for the APIs using the provided Dockerfiles. For example, to build the API gateway:
   ```
   docker build -t ru5ty-gate/api-gateway -f apps/backend/api-gateway/Dockerfile .
   ```

3. **Run with Docker Compose**:
   To run the entire application stack locally, use Docker Compose:
   ```
   docker-compose up
   ```

4. **Deploy to Kubernetes**:
   To deploy the application to a Kubernetes cluster, apply the deployment configurations:
   ```
   kubectl apply -f devops/k8s/
   ```

## n8n Workflow Automation

n8n is deployed as a separate service with per-project isolation. Each project/client gets its own:
- Dedicated n8n instance
- Separate database
- Unique encryption key
- Isolated storage volume

### Quick Start (Development)

```bash
# 1. Start the main dev infrastructure
docker compose -f devops/docker-compose.dev.yml up -d

# 2. Create n8n database
docker exec -it ru5ty-gate-postgres-dev psql -U postgres -c "CREATE DATABASE n8n_dev;"

# 3. Start n8n dev instance
cd apps/automation/n8n/compose
docker compose -f docker-compose.dev.yml --env-file .env.dev up -d

# 4. Access n8n at http://localhost:4008
```

### Creating Project-Specific Instances

```bash
# Generate configs for a new project
npx ts-node apps/automation/n8n/scripts/create-instance.ts \
  --project acme-corp \
  --domain yourdomain.co.za

# Start the instance
cd apps/automation/n8n/compose
docker compose -f docker-compose.acme-corp.yml --env-file .env.acme-corp up -d
```

See [apps/automation/n8n/README.md](../apps/automation/n8n/README.md) for complete documentation.

## Scripts

- **dev.sh**: Starts the development environment.
- **test.sh**: Runs the tests for the application.
- **migrate.sh**: Performs database migrations.

## Notes

- Ensure that you have Docker and Kubernetes set up on your machine before proceeding.
- Modify the `.env` files as necessary to configure environment variables for your local setup.

For further details on each component, refer to the respective README files in the individual API directories.
