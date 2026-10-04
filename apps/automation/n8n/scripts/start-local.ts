#!/usr/bin/env node
/**
 * =============================================================================
 * n8n Local Development Runner
 * =============================================================================
 * Runs n8n locally without Docker, connecting to local PostgreSQL
 *
 * Usage:
 *   npx ts-node start-local.ts
 *   # or
 *   node start-local.js
 *
 * Prerequisites:
 *   - Node.js 18+
 *   - PostgreSQL running locally (or via Docker)
 *   - n8n installed globally: npm install -g n8n
 * =============================================================================
 */

import { spawn } from "child_process";
import * as dotenv from "dotenv";
import * as fs from "fs";
import * as path from "path";

const scriptDir = __dirname;
const envFile = path.join(scriptDir, "..", "local", ".env.local");

if (!fs.existsSync(envFile)) {
    console.error(`Error: ${envFile} not found`);
    console.error("Copy .env.local.example to .env.local and configure it first.");
    process.exit(1);
}

dotenv.config({ path: envFile });

const requiredVars = ["N8N_ENCRYPTION_KEY", "DB_POSTGRESDB_HOST", "DB_POSTGRESDB_DATABASE", "DB_POSTGRESDB_USER", "DB_POSTGRESDB_PASSWORD"];

const missing = requiredVars.filter((v) => !process.env[v]);
if (missing.length > 0) {
    console.error("Missing required environment variables:", missing.join(", "));
    process.exit(1);
}

console.log("Starting n8n locally...");
console.log(`Host: ${process.env.N8N_HOST || "localhost"}`);
console.log(`Port: ${process.env.N8N_PORT || "4008"}`);
console.log(`Database: ${process.env.DB_POSTGRESDB_DATABASE}`);
console.log("");

const n8n = spawn("n8n", ["start"], {
    env: { ...process.env, N8N_PORT: process.env.N8N_PORT || "4008" },
    stdio: "inherit",
    shell: true,
});

n8n.on("error", (err) => {
    if (err.message.includes("ENOENT")) {
        console.error("Error: n8n is not installed globally.");
        console.error("Install it with: npm install -g n8n");
    } else {
        console.error("Error starting n8n:", err);
    }
    process.exit(1);
});

n8n.on("close", (code) => {
    process.exit(code || 0);
});

process.on("SIGINT", () => {
    n8n.kill("SIGINT");
});

process.on("SIGTERM", () => {
    n8n.kill("SIGTERM");
});
