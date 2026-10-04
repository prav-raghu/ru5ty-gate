#!/usr/bin/env npx ts-node
/**
 * =============================================================================
 * n8n Instance Creation Script
 * =============================================================================
 * Generates Docker Compose and Kubernetes configurations for a new project instance
 *
 * Usage:
 *   npx ts-node create-instance.ts --project <slug> [options]
 *
 * Options:
 *   --project, -p     Project slug (required, kebab-case)
 *   --domain, -d      Base domain (default: yourdomain.co.za)
 *   --db-host         Database host (default: postgres)
 *   --db-password     Database password (generates random if not provided)
 *   --port            n8n port (default: 5678)
 *   --docker-only     Generate only Docker Compose files
 *   --k8s-only        Generate only Kubernetes manifests
 *
 * Examples:
 *   npx ts-node create-instance.ts --project acme-corp --domain example.com
 *   npx ts-node create-instance.ts -p client-xyz --db-host postgres.internal
 * =============================================================================
 */

import * as crypto from "node:crypto";
import * as fs from "node:fs";
import * as path from "node:path";

interface InstanceConfig {
    projectSlug: string;
    projectSlugUnderscore: string;
    domain: string;
    dbHost: string;
    dbPassword: string;
    encryptionKey: string;
    adminPassword: string;
    port: number;
    dockerOnly: boolean;
    k8sOnly: boolean;
}

function parseArgs(): InstanceConfig {
    const args = process.argv.slice(2);
    const config: Partial<InstanceConfig> = {
        domain: "yourdomain.co.za",
        dbHost: "postgres",
        port: 5678,
        dockerOnly: false,
        k8sOnly: false,
    };

    for (let i = 0; i < args.length; i++) {
        const arg = args[i];
        const nextArg = args[i + 1];

        switch (arg) {
            case "--project":
            case "-p":
                config.projectSlug = nextArg;
                i++;
                break;
            case "--domain":
            case "-d":
                config.domain = nextArg;
                i++;
                break;
            case "--db-host":
                config.dbHost = nextArg;
                i++;
                break;
            case "--db-password":
                config.dbPassword = nextArg;
                i++;
                break;
            case "--port":
                config.port = Number.parseInt(nextArg, 10);
                i++;
                break;
            case "--docker-only":
                config.dockerOnly = true;
                break;
            case "--k8s-only":
                config.k8sOnly = true;
                break;
            case "--help":
            case "-h":
                printHelp();
                process.exit(0);
        }
    }

    if (!config.projectSlug) {
        console.error("Error: --project is required");
        printHelp();
        process.exit(1);
    }

    if (!/^[a-z][a-z0-9-]*$/.test(config.projectSlug)) {
        console.error("Error: Project slug must be lowercase, start with a letter, and contain only letters, numbers, and hyphens");
        process.exit(1);
    }

    config.projectSlugUnderscore = config.projectSlug.replace(/-/g, "_");
    config.encryptionKey = config.encryptionKey || crypto.randomBytes(32).toString("hex");
    config.dbPassword = config.dbPassword || crypto.randomBytes(16).toString("hex");
    config.adminPassword = crypto.randomBytes(12).toString("base64").replace(/[+/=]/g, "");

    return config as InstanceConfig;
}

function printHelp(): void {
    console.log(`
n8n Instance Creation Script

Usage:
  npx ts-node create-instance.ts --project <slug> [options]

Options:
  --project, -p     Project slug (required, kebab-case)
  --domain, -d      Base domain (default: yourdomain.co.za)
  --db-host         Database host (default: postgres)
  --db-password     Database password (generates random if not provided)
  --port            n8n port (default: 5678)
  --docker-only     Generate only Docker Compose files
  --k8s-only        Generate only Kubernetes manifests

Examples:
  npx ts-node create-instance.ts --project acme-corp --domain example.com
  npx ts-node create-instance.ts -p client-xyz --db-host postgres.internal
`);
}

function generateDockerCompose(config: InstanceConfig): string {
    return `# =============================================================================
# n8n Instance: ${config.projectSlug}
# =============================================================================
# Generated: ${new Date().toISOString()}
# =============================================================================

services:
  n8n-${config.projectSlug}:
    image: n8nio/n8n:latest
    container_name: n8n-${config.projectSlug}
    restart: unless-stopped
    ports:
      - "\${N8N_PORT:-${config.port}}:5678"
    environment:
      - N8N_HOST=\${N8N_HOST}
      - N8N_PORT=5678
      - N8N_PROTOCOL=\${N8N_PROTOCOL}
      - WEBHOOK_URL=\${WEBHOOK_URL}
      - N8N_ENCRYPTION_KEY=\${N8N_ENCRYPTION_KEY}
      - GENERIC_TIMEZONE=\${GENERIC_TIMEZONE:-Africa/Johannesburg}
      - TZ=\${TZ:-Africa/Johannesburg}
      - DB_TYPE=postgresdb
      - DB_POSTGRESDB_HOST=\${DB_POSTGRESDB_HOST}
      - DB_POSTGRESDB_PORT=\${DB_POSTGRESDB_PORT:-5432}
      - DB_POSTGRESDB_DATABASE=\${DB_POSTGRESDB_DATABASE}
      - DB_POSTGRESDB_USER=\${DB_POSTGRESDB_USER}
      - DB_POSTGRESDB_PASSWORD=\${DB_POSTGRESDB_PASSWORD}
      - N8N_BASIC_AUTH_ACTIVE=\${N8N_BASIC_AUTH_ACTIVE:-true}
      - N8N_BASIC_AUTH_USER=\${N8N_BASIC_AUTH_USER}
      - N8N_BASIC_AUTH_PASSWORD=\${N8N_BASIC_AUTH_PASSWORD}
      - EXECUTIONS_MODE=\${EXECUTIONS_MODE:-regular}
      - EXECUTIONS_PROCESS=\${EXECUTIONS_PROCESS:-main}
      - EXECUTIONS_DATA_PRUNE=\${EXECUTIONS_DATA_PRUNE:-true}
      - EXECUTIONS_DATA_MAX_AGE=\${EXECUTIONS_DATA_MAX_AGE:-168}
      - N8N_LOG_LEVEL=\${N8N_LOG_LEVEL:-info}
      - N8N_METRICS=\${N8N_METRICS:-false}
      - N8N_SECURE_COOKIE=\${N8N_SECURE_COOKIE:-true}
    volumes:
      - n8n_data_${config.projectSlug}:/home/node/.n8n
    networks:
      - n8n-${config.projectSlug}-network
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://localhost:5678/healthz || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
    depends_on:
      postgres-n8n-${config.projectSlug}:
        condition: service_healthy

  postgres-n8n-${config.projectSlug}:
    image: postgres:16-alpine
    container_name: postgres-n8n-${config.projectSlug}
    restart: unless-stopped
    environment:
      - POSTGRES_USER=\${DB_POSTGRESDB_USER}
      - POSTGRES_PASSWORD=\${DB_POSTGRESDB_PASSWORD}
      - POSTGRES_DB=\${DB_POSTGRESDB_DATABASE}
    volumes:
      - n8n_db_${config.projectSlug}:/var/lib/postgresql/data
    networks:
      - n8n-${config.projectSlug}-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U \${DB_POSTGRESDB_USER} -d \${DB_POSTGRESDB_DATABASE}"]
      interval: 5s
      timeout: 5s
      retries: 5
      start_period: 10s

volumes:
  n8n_data_${config.projectSlug}:
    name: n8n_data_${config.projectSlug}
  n8n_db_${config.projectSlug}:
    name: n8n_db_${config.projectSlug}

networks:
  n8n-${config.projectSlug}-network:
    name: n8n-${config.projectSlug}-network
    driver: bridge
`;
}

function generateEnvFile(config: InstanceConfig): string {
    return `# =============================================================================
# n8n Instance: ${config.projectSlug}
# =============================================================================
# Generated: ${new Date().toISOString()}
# =============================================================================

PROJECT_SLUG=${config.projectSlug}

# n8n Core Configuration
N8N_ENCRYPTION_KEY=${config.encryptionKey}
N8N_HOST=n8n.${config.projectSlug}.${config.domain}
N8N_PORT=${config.port}
N8N_PROTOCOL=https
WEBHOOK_URL=https://n8n.${config.projectSlug}.${config.domain}

# Timezone
GENERIC_TIMEZONE=Africa/Johannesburg
TZ=Africa/Johannesburg

# Database Configuration
DB_TYPE=postgresdb
DB_POSTGRESDB_HOST=postgres-n8n-${config.projectSlug}
DB_POSTGRESDB_PORT=5432
DB_POSTGRESDB_DATABASE=n8n_${config.projectSlugUnderscore}
DB_POSTGRESDB_USER=n8n_${config.projectSlugUnderscore}
DB_POSTGRESDB_PASSWORD=${config.dbPassword}

# Authentication
N8N_BASIC_AUTH_ACTIVE=true
N8N_BASIC_AUTH_USER=admin
N8N_BASIC_AUTH_PASSWORD=${config.adminPassword}

# Execution Settings
EXECUTIONS_MODE=regular
EXECUTIONS_PROCESS=main
EXECUTIONS_DATA_PRUNE=true
EXECUTIONS_DATA_MAX_AGE=168

# Logging
N8N_LOG_LEVEL=info

# Metrics
N8N_METRICS=false

# Security
N8N_SECURE_COOKIE=true
`;
}

function generateK8sNamespace(config: InstanceConfig): string {
    return `apiVersion: v1
kind: Namespace
metadata:
  name: ${config.projectSlug}
  labels:
    app.kubernetes.io/name: n8n
    app.kubernetes.io/instance: n8n-${config.projectSlug}
    app.kubernetes.io/component: automation
    project: ${config.projectSlug}
`;
}

function generateK8sSecret(config: InstanceConfig): string {
    const encryptionKeyBase64 = Buffer.from(config.encryptionKey).toString("base64");
    const dbPasswordBase64 = Buffer.from(config.dbPassword).toString("base64");
    const adminUserBase64 = Buffer.from("admin").toString("base64");
    const adminPasswordBase64 = Buffer.from(config.adminPassword).toString("base64");

    return `apiVersion: v1
kind: Secret
metadata:
  name: n8n-secrets
  namespace: ${config.projectSlug}
  labels:
    app.kubernetes.io/name: n8n
    app.kubernetes.io/instance: n8n-${config.projectSlug}
type: Opaque
data:
  N8N_ENCRYPTION_KEY: ${encryptionKeyBase64}
  DB_POSTGRESDB_PASSWORD: ${dbPasswordBase64}
  N8N_BASIC_AUTH_USER: ${adminUserBase64}
  N8N_BASIC_AUTH_PASSWORD: ${adminPasswordBase64}

---
apiVersion: v1
kind: Secret
metadata:
  name: n8n-db-credentials
  namespace: ${config.projectSlug}
  labels:
    app.kubernetes.io/name: n8n
    app.kubernetes.io/instance: n8n-${config.projectSlug}
type: Opaque
stringData:
  host: "postgres.database.svc.cluster.local"
  port: "5432"
  database: "n8n_${config.projectSlugUnderscore}"
  username: "n8n_${config.projectSlugUnderscore}"
`;
}

function generateK8sConfigMap(config: InstanceConfig): string {
    return `apiVersion: v1
kind: ConfigMap
metadata:
  name: n8n-config
  namespace: ${config.projectSlug}
  labels:
    app.kubernetes.io/name: n8n
    app.kubernetes.io/instance: n8n-${config.projectSlug}
data:
  N8N_HOST: "n8n.${config.projectSlug}.${config.domain}"
  N8N_PORT: "5678"
  N8N_PROTOCOL: "https"
  WEBHOOK_URL: "https://n8n.${config.projectSlug}.${config.domain}"
  GENERIC_TIMEZONE: "Africa/Johannesburg"
  TZ: "Africa/Johannesburg"
  DB_TYPE: "postgresdb"
  DB_POSTGRESDB_PORT: "5432"
  EXECUTIONS_MODE: "regular"
  EXECUTIONS_PROCESS: "main"
  EXECUTIONS_DATA_PRUNE: "true"
  EXECUTIONS_DATA_MAX_AGE: "168"
  N8N_LOG_LEVEL: "info"
  N8N_METRICS: "true"
  N8N_BASIC_AUTH_ACTIVE: "true"
  N8N_SECURE_COOKIE: "true"
`;
}

function generateK8sIngress(config: InstanceConfig): string {
    return `apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: n8n
  namespace: ${config.projectSlug}
  labels:
    app.kubernetes.io/name: n8n
    app.kubernetes.io/instance: n8n-${config.projectSlug}
  annotations:
    kubernetes.io/ingress.class: nginx
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "50m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
    nginx.ingress.kubernetes.io/proxy-http-version: "1.1"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  tls:
    - hosts:
        - n8n.${config.projectSlug}.${config.domain}
      secretName: n8n-${config.projectSlug}-tls
  rules:
    - host: n8n.${config.projectSlug}.${config.domain}
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: n8n
                port:
                  number: 80
`;
}

function generateK8sKustomization(config: InstanceConfig): string {
    return `apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

namespace: ${config.projectSlug}

resources:
  - namespace.yaml
  - secret.yaml
  - configmap.yaml
  - pvc.yaml
  - deployment.yaml
  - service.yaml
  - ingress.yaml

commonLabels:
  app.kubernetes.io/managed-by: kustomize
  project: ${config.projectSlug}
`;
}

function copyBaseK8sFiles(sourceDir: string, targetDir: string, config: InstanceConfig): void {
    const filesToCopy = ["deployment.yaml", "service.yaml", "pvc.yaml"];

    for (const file of filesToCopy) {
        const sourcePath = path.join(sourceDir, file);
        const targetPath = path.join(targetDir, file);

        if (fs.existsSync(sourcePath)) {
            let content = fs.readFileSync(sourcePath, "utf-8");
            content = content.replace(/<PROJECT_SLUG>/g, config.projectSlug);
            content = content.replace(/<PROJECT_SLUG_UNDERSCORE>/g, config.projectSlugUnderscore);
            content = content.replace(/yourdomain\.co\.za/g, config.domain);
            fs.writeFileSync(targetPath, content);
        }
    }
}

function main(): void {
    const config = parseArgs();
    const scriptDir = __dirname;
    const composeDir = path.join(scriptDir, "..", "compose");
    const k8sBaseDir = path.join(scriptDir, "..", "k8s");
    const k8sProjectDir = path.join(k8sBaseDir, config.projectSlug);

    console.log("\n=== n8n Instance Generator ===\n");
    console.log(`Project: ${config.projectSlug}`);
    console.log(`Domain: ${config.domain}`);
    console.log(`URL: https://n8n.${config.projectSlug}.${config.domain}`);
    console.log("");

    if (!config.k8sOnly) {
        console.log("Generating Docker Compose files...");

        const composePath = path.join(composeDir, `docker-compose.${config.projectSlug}.yml`);
        const envPath = path.join(composeDir, `.env.${config.projectSlug}`);

        fs.writeFileSync(composePath, generateDockerCompose(config));
        fs.writeFileSync(envPath, generateEnvFile(config));

        console.log(`  ✓ ${composePath}`);
        console.log(`  ✓ ${envPath}`);
    }

    if (!config.dockerOnly) {
        console.log("\nGenerating Kubernetes manifests...");

        if (!fs.existsSync(k8sProjectDir)) {
            fs.mkdirSync(k8sProjectDir, { recursive: true });
        }

        fs.writeFileSync(path.join(k8sProjectDir, "namespace.yaml"), generateK8sNamespace(config));
        fs.writeFileSync(path.join(k8sProjectDir, "secret.yaml"), generateK8sSecret(config));
        fs.writeFileSync(path.join(k8sProjectDir, "configmap.yaml"), generateK8sConfigMap(config));
        fs.writeFileSync(path.join(k8sProjectDir, "ingress.yaml"), generateK8sIngress(config));
        fs.writeFileSync(path.join(k8sProjectDir, "kustomization.yaml"), generateK8sKustomization(config));

        copyBaseK8sFiles(k8sBaseDir, k8sProjectDir, config);

        console.log(`  ✓ ${k8sProjectDir}/namespace.yaml`);
        console.log(`  ✓ ${k8sProjectDir}/secret.yaml`);
        console.log(`  ✓ ${k8sProjectDir}/configmap.yaml`);
        console.log(`  ✓ ${k8sProjectDir}/ingress.yaml`);
        console.log(`  ✓ ${k8sProjectDir}/kustomization.yaml`);
        console.log(`  ✓ ${k8sProjectDir}/deployment.yaml`);
        console.log(`  ✓ ${k8sProjectDir}/service.yaml`);
        console.log(`  ✓ ${k8sProjectDir}/pvc.yaml`);
    }

    console.log("\n=== Generated Credentials ===\n");
    console.log("⚠️  SAVE THESE VALUES SECURELY - THEY CANNOT BE RECOVERED!\n");
    console.log(`Encryption Key: ${config.encryptionKey}`);
    console.log(`Database Password: ${config.dbPassword}`);
    console.log(`Admin Password: ${config.adminPassword}`);
    console.log("");

    console.log("=== Next Steps ===\n");

    if (!config.k8sOnly) {
        console.log("Docker Compose:");
        console.log(`  cd apps/automation/n8n/compose`);
        console.log(`  docker compose -f docker-compose.${config.projectSlug}.yml --env-file .env.${config.projectSlug} up -d`);
        console.log("");
    }

    if (!config.dockerOnly) {
        console.log("Kubernetes:");
        console.log(`  kubectl apply -k apps/automation/n8n/k8s/${config.projectSlug}/`);
        console.log("");
    }
}

main();
