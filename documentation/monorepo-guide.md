# Ru5ty Gate Monorepo Guide

How to run, build, test and deploy everything in this repository. The captive portal agent has its own guide in [apps/backend/agent/README.md](../apps/backend/agent/README.md); this guide covers the shared platform (Rust APIs, frontends, mobile, CMS and automation) it lives alongside.

<p align="left">
<img src="https://img.shields.io/badge/Rust-1.97-orange" />
<img src="https://img.shields.io/badge/Axum-0.8-black" />
<img src="https://img.shields.io/badge/SQLx-0.9-blue" />
<img src="https://img.shields.io/badge/Next.js-15-black" />
<img src="https://img.shields.io/badge/React-19-blue" />
<img src="https://img.shields.io/badge/Turborepo-Latest-red" />
<img src="https://img.shields.io/badge/pnpm-11-orange" />
</p>

## Architecture

This monorepo is structured to support multiple backend APIs, frontend applications, mobile apps, and automation services, all sharing common packages for consistency and code reuse.

```mermaid
flowchart TD

    A[Customer Web]
    B[Admin Web]
    C[Mobile App]

    A --> D[API Gateway]
    B --> D
    C --> D

    D --> E[Customer API]
    D --> F[Admin API]
    D --> G[Schedule API]

    E --> H[(PostgreSQL)]
    F --> H
    G --> H

    E --> I[(Redis)]
    F --> I
    G --> I

    J[Strapi CMS] --> D
    K[n8n] --> D
```

## Getting Started (local development)

Local development always runs the four Rust services with `cargo` and the frontends with `pnpm`, so you get fast rebuilds and plain log output. The only choice is how Postgres and Redis are provided:

| | Option A: Docker | Option B: Native install |
|---|---|---|
| Postgres 16 and Redis 7 | Containers from `devops/docker-compose.dev.yml` | Installed on the machine |
| Extra tools included | Adminer (DB UI), Redis Commander (Redis UI), OpenObserve | none |
| Needs | Docker Desktop | PostgreSQL and Redis installers (Redis has no native Windows build: Memurai or WSL2) |
| Best when | You want a disposable, identical setup | You cannot or do not want to run Docker |

Steps 1, 2 and 4 to 8 are the same for both. Only step 3 differs. Commands are PowerShell; on macOS or Linux, `cp` replaces `Copy-Item` and `export` replaces `$env:`.

### 1. Install the prerequisites

Needed for both options:

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.97.0 (pinned in `rust-toolchain.toml`, installed automatically on first `cargo` run) | `winget install Rustlang.Rustup` |
| MSVC build tools | any recent | `winget install Microsoft.VisualStudio.2022.BuildTools`, then tick **Desktop development with C++** (Rust cannot link on Windows without it) |
| Node.js | 22+ | `winget install OpenJS.NodeJS.LTS` |
| pnpm | 11 (pinned by `packageManager`) | `corepack enable` |
| Git Bash | optional | Only needed for `pnpm dev:backend` (see step 6) |

Plus, depending on the option:

| Option | Tool | Install |
|--------|------|---------|
| A: Docker | Docker Desktop (with the WSL2 backend) | `winget install Docker.DockerDesktop`, then start it and wait for the engine to be running |
| B: Native | PostgreSQL 16+ | `winget install PostgreSQL.PostgreSQL.16` (keep port 5432 and remember the `postgres` password you set) |
| B: Native | Redis 7+ | [Memurai](https://www.memurai.com/) (`winget install Memurai.MemuraDeveloper`) or Redis inside WSL2 (`sudo apt install redis-server && sudo service redis-server start`) |

Open a new terminal after installing so the new `PATH` entries are picked up, then check:

```powershell
rustc --version; cargo --version; node --version; pnpm --version
docker --version    # Option A
psql --version      # Option B (if not found, add C:\Program Files\PostgreSQL\16\bin to PATH)
```

### 2. Clone and install JavaScript dependencies

```powershell
git clone <repository-url>
cd ru5ty-gate
pnpm install
```

### 3. Start Postgres and Redis

#### Option A: Docker

```powershell
docker compose -f devops/docker-compose.dev.yml up -d postgres redis
docker compose -f devops/docker-compose.dev.yml ps
```

Wait until both show `healthy`. The database `ru5ty_gate` is created automatically with user `postgres` and password `postgres`, which is exactly what the `.env.example` files expect, so there is nothing else to configure.

To also start the web UIs, drop the service names to bring up the whole dev stack:

```powershell
docker compose -f devops/docker-compose.dev.yml up -d
```

| Service | URL | Notes |
|---------|-----|-------|
| Adminer | http://localhost:8080 | System `PostgreSQL`, server `postgres`, user `postgres`, password `postgres`, database `ru5ty_gate` |
| Redis Commander | http://localhost:8081 | Browse keys |
| OpenObserve | http://localhost:5080 | Log and trace viewer. Login `admin@example.com` / `Dev_password1!` |

Day-to-day:

```powershell
docker compose -f devops/docker-compose.dev.yml stop            # stop, keep data
docker compose -f devops/docker-compose.dev.yml start           # start again
docker compose -f devops/docker-compose.dev.yml down -v         # delete containers AND all data (fresh database)
```

Port 5432 or 6379 already in use? A native Postgres or Redis is already running. Stop it, or change the left side of the port mapping in the compose file and the matching `DATABASE_URL` / `REDIS_URL` in step 4.

#### Option B: Native install

Make sure the PostgreSQL service and Redis are running, then create the database:

```powershell
psql -U postgres -h localhost -c "CREATE DATABASE ru5ty_gate;"
```

The `.env.example` files assume user `postgres`, password `postgres`, database `ru5ty_gate`. If your `postgres` password is different, change it in the `DATABASE_URL` lines in step 4.

Redis is optional to start the services: if it is unreachable they log `Redis unavailable - running without cache` and carry on. Run it anyway, since the token and rate-limit features use it.

### 4. Create the environment files

Each service reads its own `.env` from its own folder. Copy every example:

```powershell
foreach ($service in "admin-api", "customer-api", "schedule-api", "api-gateway") {
    Copy-Item "apps\backend\$service\.env.example" "apps\backend\$service\.env"
}
Copy-Item apps\frontend\admin-web\.env.example apps\frontend\admin-web\.env
Copy-Item apps\frontend\customer-web\.env.example apps\frontend\customer-web\.env
```

Then replace the placeholder secrets. The `REPLACE_WITH_...` values in the examples are not usable as-is. This helper prints cryptographically random hex:

```powershell
function New-HexSecret([int]$Bytes) {
    $buffer = New-Object byte[] $Bytes
    [Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($buffer)
    -join ($buffer | ForEach-Object { $_.ToString("x2") })
}

New-HexSecret 64   # JWT_SECRET and JWT_REFRESH_SECRET (use different values; at least 32 characters)
New-HexSecret 32   # TWO_FACTOR_ENCRYPTION_KEY (must be exactly 64 hex characters)
New-HexSecret 32   # SCHEDULE_API_KEY (at least 32 characters)
```

| File | Values you must set |
|------|---------------------|
| `apps/backend/admin-api/.env` | `JWT_SECRET`, `JWT_REFRESH_SECRET`, `TWO_FACTOR_ENCRYPTION_KEY`, and `DATABASE_URL` / `REDIS_URL` if yours differ |
| `apps/backend/customer-api/.env` | `JWT_SECRET`, `JWT_REFRESH_SECRET` (admin-api and customer-api can use different secrets) |
| `apps/backend/schedule-api/.env` | `SCHEDULE_API_KEY` |
| `apps/backend/api-gateway/.env` | nothing: the defaults point at ports 4001-4003 |

Email is optional locally. Without `MAILTRAP_API_KEY` the services start, but onboarding, verification and password-reset emails are not delivered. Email goes through the Mailtrap API (not SMTP), so there is no local mail catcher. Set `MAILTRAP_API_KEY` and `MAILTRAP_TEST_INBOX_ID` to have outgoing mail land in a Mailtrap sandbox inbox.

**Every app runs in the 4000 range in development, one fixed port each:**

| Port | App | Port | App |
|------|-----|------|-----|
| 4000 | api-gateway | 4005 | customer-web (Next.js) |
| 4001 | admin-api | 4006 | cms (Strapi) |
| 4002 | customer-api | 4007 | customer-mobile (Expo Metro) |
| 4003 | schedule-api | 4008 | n8n (dev compose host port) |
| 4004 | admin-web (Vite) | | |

The ports are fixed on purpose. The Vite apps use `strictPort`, Next.js is started with `-p` and Expo with `--port 4007`, so if a port is taken the app fails to start instead of silently moving to another one (which would break CORS). `CORS_ORIGIN` in `api-gateway/.env` and `customer-api/.env` already points at `http://localhost:4005`; it accepts a comma-separated list if more than one web origin is needed. Native mobile requests carry no browser origin, so CORS does not apply to the Expo app. The `.env.example` files carry the same values. Production containers keep their own ports; see `CLAUDE.md`.

### 5. Run the migrations

Migrations are never applied automatically at startup. Run them from the `admin-api` folder so it picks up that folder's `.env`:

```powershell
cd apps\backend\admin-api
cargo run --bin admin-api -- migrate
cd ..\..\..
```

The first build compiles the whole workspace and takes several minutes. Expect `Migrations applied`. Roles and user statuses are seeded by the migrations.

### 6. Start the backend

Run each service from its own folder (that is how its `.env` is found). Use four PowerShell terminals:

```powershell
cd apps\backend\admin-api;    cargo run --bin admin-api
cd apps\backend\customer-api; cargo run --bin customer-api
cd apps\backend\schedule-api; cargo run --bin schedule-api
cd apps\backend\api-gateway;  cargo run --bin api-gateway
```

Or start all four in one terminal. `pnpm dev:backend` runs `bash devops/scripts/dev.sh`, and on Windows a plain `bash` is often the WSL launcher (which has no Rust), so call Git Bash explicitly instead:

```powershell
& "C:\Program Files\Git\bin\bash.exe" devops/scripts/dev.sh
```

Each service prints a startup line once it is listening, for example `admin-api v1.0.0 running on http://localhost:4001 (development)`.

Check they are up:

```powershell
curl.exe http://localhost:4001/api/v1/ping
```

### 7. Create the first admin account

The first Super Admin is created once, through `admin-api`'s bootstrap route (`ADMIN_BOOTSTRAP_ENABLED=true` in its `.env`, which is the default):

```powershell
curl.exe -X POST http://localhost:4001/api/v1/auth/bootstrap-admin `
  -H "Content-Type: application/json" `
  -d '{\"username\":\"admin\",\"email\":\"you@example.com\",\"password\":\"ChangeMe12345\"}'
```

The route locks itself permanently after the first successful call. Then set `ADMIN_BOOTSTRAP_ENABLED=false` in `admin-api/.env` and restart it.

### 8. Start the frontends

```powershell
pnpm dev:admin-web      # http://localhost:4004
pnpm dev:customer-web   # http://localhost:4005
pnpm dev:mobile         # Expo Metro dev server on port 4007
```

#### Running the mobile app (Expo)

`pnpm dev:mobile` starts the Expo dev server on **4007** and prints a QR code. Press `a` for an Android emulator, or scan the code with the Expo Go app on a phone on the same network. iOS builds need macOS.

The app reads the API address from `EXPO_PUBLIC_MOBILE_API_BASE_URL` (copy `apps/mobile/customer-mobile/.env.example` to `.env`). `localhost` only works in an iOS simulator:

| Where the app runs | `EXPO_PUBLIC_MOBILE_API_BASE_URL` |
|--------------------|----------------------------------|
| iOS simulator | `http://localhost:4000` |
| Android emulator | `http://10.0.2.2:4000` |
| Physical device | `http://<this machine's LAN IP>:4000` |

For a physical device, allow inbound connections on ports 4007 and 4000 in Windows Firewall. Restart the dev server after changing `.env`. `pnpm --filter customer-mobile build` bundles the iOS and Android JavaScript as a check; it does not produce an installable app (that is an EAS build). See `.claude/agents/mobile.md`.

### Debugging with F5 (VS Code)

Open the repository in VS Code and install the recommended extensions when prompted (the Rust ones are CodeLLDB `vadimcn.vscode-lldb` and `rust-lang.rust-analyzer`). Then open **Run and Debug** (Ctrl+Shift+D), pick a launch configuration and press F5.

| Launch configuration | What starts |
|----------------------|-------------|
| 💻 Full Dev Environment | api-gateway, admin-api, customer-api, schedule-api, admin-web, customer-web, customer-mobile (ports 4000-4005 and 4007) |
| 🛠️ Admin Stack (API + Web) | admin-api and admin-web |
| 🌐 Customer Stack (API + Web) | customer-api and customer-web |
| 🔥 Full Backend Stack (APIs only) | the four Rust services |
| 💻 Full Dev + CMS | everything above plus Strapi (needs `npm install` and a `.env` in `apps/cms` first) |
| One per app | 🌐 API Gateway, 🔐 Admin API, 👤 Customer API, 📅 Schedule API, 🛠️ Admin Web, 🌐 Customer Web, 📱 Customer Mobile, 📰 CMS |
| 🧪 Debug Rust Tests | builds the workspace tests under the debugger |

How it works:

- A compound first builds the Rust binaries it needs (task `🔨 Build: ...`) with `CARGO_PROFILE_DEV_DEBUG=true`, so variables are inspectable. The repo's normal dev profile only keeps line tables. Switching between debug and normal builds recompiles the workspace crates (not the dependencies).
- Each Rust service runs from its own folder with its own `.env`, and breakpoints in Rust code work.
- The web configs run `pnpm dev:*` in the integrated terminal. Node breakpoints work (Next.js server code, Vite config), and when the dev server prints its URL, VS Code opens Edge attached to the debugger so breakpoints in browser code work too.
- 📱 Customer Mobile runs the Expo dev server in the terminal (press `a` for an Android emulator, or scan the QR code with Expo Go). There is no browser debugger because it is not a web app.
- Stopping any configuration stops the whole compound.
- Postgres and Redis must already be running (step 3), the `.env` files must exist (step 4), and the database must be migrated (step 5). The tasks `🗄️ DB: Create ru5ty_gate` and `🗄️ DB: Migrate` (Terminal → Run Task) do the database part once. Migrations are never run by F5.

### Troubleshooting

| Symptom | Cause and fix |
|---------|---------------|
| `cargo` is not recognized in the VS Code terminal but works in `cmd` | VS Code reads `PATH` once at startup, so it does not see tools installed after it was opened. Close **all** VS Code windows (File → Exit) and reopen. F5 and the build tasks need this too. Quick fix for one terminal only: `$env:Path = [Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [Environment]::GetEnvironmentVariable("Path","User")` |
| `cargo fmt` fails with `Incorrect newline style` | The checkout has CRLF line endings (Git for Windows `core.autocrlf=true`). The repository enforces LF through `.gitattributes`, `rustfmt.toml` and `.editorconfig`. Re-checkout so files are written as LF, or run `cargo fmt --all` to rewrite the Rust files. |
| F5 fails with `Internal debugger error: Error parsing line: '...'` | A value in a service `.env` contains spaces and is not quoted. Quote it, for example `MAILTRAP_FROM_NAME="Your App Name"`. Unquoted values also stop the services loading the rest of that file. |
| `linker 'link.exe' not found` | MSVC build tools are missing. Install them (step 1) and open a new terminal. |
| Service exits immediately with `invalid configuration` | A required variable is missing or still a placeholder. The error names the variable. |
| `password authentication failed for user "postgres"` | `DATABASE_URL` has the wrong password. |
| `Redis unavailable - running without cache` | Redis is not running or `REDIS_URL` is wrong. The service still starts. |
| `connection refused` on port 5432 (native) | The PostgreSQL Windows service is stopped. Start it from `services.msc`. |
| `connection refused` on port 5432 (Docker) | The container is not up yet or Docker Desktop is not running. Check `docker compose -f devops/docker-compose.dev.yml ps`. |
| `error during connect` / `docker: command not found` | Docker Desktop is not installed or not started. |
| `port is already allocated` | A native Postgres or Redis already uses 5432 / 6379. Stop it or remap the port (step 3, Option A). |
| Database credentials rejected after switching option | Native Postgres and the Docker one are different servers with different data. Make sure `DATABASE_URL` points at the one you started. |
| Browser CORS error from customer-web | `CORS_ORIGIN` in `customer-api/.env` does not match customer-web's port (step 4). |
| `pnpm dev:backend` fails with `cargo: command not found` | `bash` on `PATH` is the WSL launcher, not Git Bash. Use the Git Bash command in step 6, or run the four services by hand. |

### Running the services as containers

The `.env`-driven `cargo run` flow above is the supported way to develop. For completeness, here is what the Docker files in the repo are for:

- **Each service has its own Dockerfile** (`apps/backend/<service>/Dockerfile`, `apps/frontend/<app>/Dockerfile`). Build from the repository root so `common/` is in the build context. This compiles in release mode inside Docker, so expect a long first build:

  ```powershell
  docker build -t admin-api -f apps/backend/admin-api/Dockerfile .
  docker run --rm -e DATABASE_URL="postgresql://postgres:postgres@host.docker.internal:5432/ru5ty_gate" admin-api migrate
  ```

  `host.docker.internal` lets a container reach Postgres published on your machine (Option A or B).

- **The root `docker-compose.yaml` is the production stack, not a local one.** It pulls prebuilt images from `ghcr.io`, runs with `APP_ENV=production`, joins an external `coolify` network, and contains no Postgres or Redis. Use it only through Coolify (see Deployment Readiness below). Running `docker compose up` on a laptop will fail on the missing `coolify` network.

- **`devops/docker-compose.nginx.yml` is stale.** It builds from `devops/docker/*.Dockerfile`, which no longer exist.

### Available Scripts

| Script | Description |
|--------|-------------|
| `pnpm dev:backend` | Start api-gateway (4000), admin-api (4001), customer-api (4002) and schedule-api (4003) |
| `pnpm dev:customer-web` | Start Customer Web (Next.js) |
| `pnpm dev:admin-web` | Start Admin Web (Vite) |
| `pnpm build` | Build the frontends |
| `cargo build --workspace` | Build the Rust backend |
| `pnpm test` | Run frontend tests |
| `pnpm test:rust` | Run Rust tests (needs `DATABASE_URL`, optional `TEST_REDIS_URL`) |
| `pnpm lint` | ESLint and `cargo clippy -D warnings` |
| `pnpm typecheck` | `tsc` for frontends and `cargo check` for Rust |
| `pnpm format` | Prettier and `cargo fmt` |
| `pnpm n8n:local` | Start n8n locally (no Docker) |
| `pnpm n8n:create-instance` | Create new n8n project instance |

## 🤖 Workflow Automation (n8n)

This repository includes n8n for workflow automation with per-project isolation. Each project/client gets its own dedicated n8n instance.

### Quick Start

```bash
# Option 1: Docker (recommended)
docker compose -f apps/automation/n8n/compose/docker-compose.dev.yml up -d

# Option 2: Local (no Docker)
npm install -g n8n
pnpm n8n:local

# Option 3: Quick test (SQLite, no setup)
n8n start
```

### Creating Project Instances

```bash
pnpm n8n:create-instance -- --project acme-corp --domain yourdomain.co.za
```

See [apps/automation/n8n/README.md](../apps/automation/n8n/README.md) for detailed documentation.

##  Package Management

The backend and `common/` crates are a Cargo workspace (`Cargo.toml` at the root). Frontends and mobile apps are pnpm workspaces.

### Adding Dependencies

```bash
# Rust: add to a specific crate
cargo add -p ru5ty-gate-customer-api serde_json

# Frontend: add to a specific app
pnpm --filter admin-web add axios

# Root tooling (dev dependency)
pnpm add -D -w prettier
```

##  Naming Conventions

This repository follows strict naming conventions for consistency:

| Context | Convention | Example |
|---------|------------|---------|
| Database tables/columns | snake_case | `user_profiles`, `created_at` |
| API responses | camelCase | `userId`, `createdAt` |
| TypeScript variables/functions | camelCase | `getUserById`, `isActive` |
| Rust functions/modules/files | snake_case | `get_user_by_id`, `user_route.rs` |
| Rust structs/enums/traits | PascalCase | `UserService`, `CreateUserRequest` |
| TypeScript classes/interfaces | PascalCase | `UserService`, `CreateUserDto` |
| React components | PascalCase | `UserProfile.tsx` |
| Constants | UPPER_SNAKE_CASE | `API_BASE_URL` |

Rust structs map snake_case fields to camelCase JSON with `#[serde(rename_all = "camelCase")]`.

For Rust and TypeScript conventions, see [rust.instructions.md](../.claude/instructions/rust.instructions.md) and [typescript.instructions.md](../.claude/instructions/typescript.instructions.md).

##  Testing

Run backend tests (a real Postgres and optionally Redis are needed; each test gets its own database through `#[sqlx::test]`):

```powershell
$env:DATABASE_URL = "postgres://postgres:postgres@127.0.0.1:5432/postgres"
$env:TEST_REDIS_URL = "redis://127.0.0.1:6379"

cargo test --workspace
cargo test -p ru5ty-gate-customer-api
```

The database user needs permission to create databases (the `postgres` superuser has it). Tests do not touch `ru5ty_gate`; each test creates and drops its own database. In bash, use `export DATABASE_URL=...` instead of `$env:`.

Frontend tests:

```bash
pnpm test
pnpm test:coverage
```

## Building

Build all packages:

```bash
pnpm build
```

##  Docker & Deployment

### Development

See [Getting Started](#getting-started-local-development) for the Docker and non-Docker setups. In short: `docker compose -f devops/docker-compose.dev.yml up -d postgres redis` for the infrastructure containers, or native installs of Postgres and Redis.

### Production

```bash
# Build and start the production stack from the root compose file
docker compose up -d
```

### Kubernetes

```bash
kubectl apply -f devops/k8s/
```


## Deployment Readiness

This section covers the three primary deployment paths supported by this repository:

- **VPS Production** (Hetzner Cloud, OVH, or any bare-metal/VPS provider)
- **Coolify** — self-hosted PaaS for GitHub-connected CI/CD
- **Local Testing** — Ubuntu on VirtualBox before touching a live server

---

### VPS Deployment (Hetzner, OVH, Bare-Metal)

#### Recommended Server Specs

| Workload | RAM | CPU | Storage | Provider Examples |
|----------|-----|-----|---------|-------------------|
| Starter (1-2 APIs, 1 frontend) | 4–8 GB | 2 vCPU | 40 GB SSD | OVH VPS-1/2, Hetzner CX22 |
| Standard (all services, Docker) | 12–16 GB | 4 vCPU | 80 GB SSD | OVH VPS-3, Hetzner CX32/CX42 |
| ML/CV workloads (e.g. YOLOv8) | 16–32 GB | 6–8 vCPU | 100 GB SSD | Dedicated/bare-metal |

> **ZAR note 🇿🇦:** Hetzner CX32 (8 GB RAM, 4 vCPU) and OVH VPS-3 (8 GB RAM) are solid entry points for full-stack production. Hetzner generally offers better RAM-per-rand value.

#### Server Prep Checklist

```bash
# Update and harden
sudo apt update && sudo apt upgrade -y
sudo apt install -y ufw fail2ban curl git

# Firewall — allow SSH, HTTP, HTTPS only
sudo ufw allow OpenSSH
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable

# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# Install Docker Compose
sudo apt install -y docker-compose-plugin
```

#### Environment Variables

Copy and configure your production `.env` before deploying:

```bash
cp devops/.env.example devops/.env.production
# Set all DATABASE_URL, REDIS_URL, JWT secrets, SMTP credentials, etc.
```

> Never commit `.env.production` to source control. Use Coolify secrets or a secrets manager in CI/CD.

#### Production Stack

```bash
# Build and start the full production stack
docker compose up -d

# Check all services are healthy
docker compose ps
```

#### Nginx Reverse Proxy

Nginx config lives in `infrastructure/nginx/`. Each app (API gateway, admin-web, customer-web) gets its own `server` block pointing to the relevant Docker container port.

```nginx
# Example: api-gateway on port 3000
server {
    listen 80;
    server_name api.yourdomain.co.za;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

Use [Certbot](https://certbot.eff.org/) for free SSL via Let's Encrypt:

```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d api.yourdomain.co.za
```

---

### Coolify (Self-Hosted PaaS)

[Coolify](https://coolify.io/) is the recommended CI/CD layer for this repository — GitHub-connected, Docker-native, and free to self-host.

#### Installing Coolify on Your VPS

```bash
curl -fsSL https://cdn.coollabs.io/coolify/install.sh | bash
```

Coolify runs on port `8000` by default. Access it via `http://<your-vps-ip>:8000` and complete the setup wizard.

> Add your VPS IP to your firewall rules temporarily during setup, then lock it down once a domain is attached.

#### Connecting Your Repository

1. In Coolify, go to **Sources** → **Add GitHub App** (or use a deploy key)
2. Select the `ru5ty-gate` repository
3. Set up auto-deploy on push to `main` (or your target branch)

#### Deploying a Service (Monorepo Pattern)

Each app in `apps/` is deployed as a separate Coolify service. Key settings:

| Setting | Value |
|---------|-------|
| **Build Pack** | `Dockerfile` |
| **Base Directory** | `/` _(always root — never the app subfolder)_ |
| **Dockerfile Location** | `apps/backend/<service-name>/Dockerfile` |
| **Port** | Match the app's exposed port |

> **Base Directory must always be `/`** — setting it to the app subfolder loses access to `common/`, `turbo.json`, and `pnpm-workspace.yaml`, which breaks the monorepo build.

#### Next.js Standalone Build (customer-web)

The `customer-web` Next.js app uses standalone output for Docker efficiency. The Dockerfile must account for the monorepo path structure:

```dockerfile
# Correct COPY paths for Next.js standalone in a monorepo
COPY --from=builder /app/apps/frontend/customer-web/.next/standalone ./
COPY --from=builder /app/apps/frontend/customer-web/.next/static ./apps/frontend/customer-web/.next/static
COPY --from=builder /app/apps/frontend/customer-web/public ./apps/frontend/customer-web/public

CMD ["node", "apps/frontend/customer-web/server.js"]
```

In `next.config.ts`, set `outputFileTracingRoot` to the monorepo root:

```typescript
const nextConfig: NextConfig = {
  output: 'standalone',
  outputFileTracingRoot: path.join(__dirname, '../../..'),
};
```

#### Coolify Environment Secrets

Set secrets per-service in Coolify's **Environment Variables** UI — these override anything in `.env` files and are injected at runtime. Never bake secrets into your Docker image.

#### Auto-Deploy Troubleshooting

If auto-deploy doesn't trigger on push to `main`:

1. Verify the GitHub webhook is registered — Coolify → Source → Webhooks
2. Check that the branch name in Coolify matches exactly (case-sensitive)
3. Re-save the service to force webhook re-registration

---

### Local Testing with Ubuntu on VirtualBox

Testing against a real Linux environment before hitting a live VPS catches Docker networking issues, permission mismatches, and env config problems early.

#### VirtualBox Setup

1. Download [VirtualBox](https://www.virtualbox.org/) and the [Ubuntu Server 24.04 LTS ISO](https://ubuntu.com/download/server)
2. Create a new VM:

| Setting | Recommended Value |
|---------|-------------------|
| RAM | 4–8 GB |
| CPUs | 2–4 |
| Storage | 40 GB (dynamically allocated) |
| Network | **Bridged Adapter** _(so the VM gets a LAN IP reachable from your host)_ |

> Set the network adapter to **Bridged** — NAT will hide the VM behind your host and make it harder to test Nginx/SSL flows.

#### VM Prep

After Ubuntu install, run the same server prep steps as the VPS:

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y docker.io docker-compose-plugin git curl ufw

sudo usermod -aG docker $USER
newgrp docker
```

#### Clone and Run

```bash
git clone <your-repo-url> ru5ty-gate
cd ru5ty-gate

cp devops/.env.example devops/.env
# Edit .env — use localhost or the VM's LAN IP for service URLs

docker compose -f devops/docker-compose.dev.yml up -d
```

#### Accessing Services from Your Host Machine

Once the VM is on a bridged network, find its IP:

```bash
# On the VM
ip a | grep inet
```

Then access services from your dev machine:

```
http://<vm-lan-ip>:3000   # API Gateway
http://<vm-lan-ip>:8000   # Coolify (if installed)
```

#### Simulating Coolify Locally

Install Coolify on the VirtualBox VM using the same install script as production:

```bash
curl -fsSL https://cdn.coollabs.io/coolify/install.sh | bash
```

This lets you test the full Coolify → GitHub webhook → Docker build → deploy pipeline before touching your real VPS.

#### SSH from Host to VM

```bash
# On the VM, ensure SSH is running
sudo apt install -y openssh-server
sudo systemctl enable --now ssh

# From your host
ssh <vm-username>@<vm-lan-ip>
```

> Add your SSH public key to `~/.ssh/authorized_keys` on the VM to avoid typing a password every time.

---

### Deployment Path Summary

```
Local Dev (pnpm dev:*)
        │
        ▼
VirtualBox Ubuntu VM
  ├── Docker Compose (dev stack)
  └── Coolify (simulate full pipeline)
        │
        ▼
VPS Production (Hetzner / OVH / Bare-Metal)
  ├── Docker Compose (prod stack)
  ├── Coolify (CI/CD + secrets)
  └── Nginx + Certbot (reverse proxy + SSL)
```


##  Documentation

| Document | Description |
|----------|-------------|
| [API Versioning](../.claude/instructions/api-versioning.instruction.md) | API versioning strategy |
| [File Upload](file-upload.md) | File upload handling |
| [Export](../.claude/instructions/export.instruction.md) | PDF/Excel export guide |
| [Quality Gates](../.claude/instructions/quality-gates.instruction.md) | Code quality standards |
| [Repository layer](repository-layer.md) | Shared data-access layer in `common/database` |
| [Rust migration](rust-migration/plan.md) | Node to Rust migration plan and wire contract |
| [Captive portal agent](../apps/backend/agent/README.md) | Router-side agent behind openNDS |
| [n8n Automation](../apps/automation/n8n/README.md) | Workflow automation setup |
| [DevOps](../devops/README.md) | Docker & Kubernetes setup |

## Contributing

1. Create a feature branch from `main`
2. Make your changes following the coding conventions
3. Write/update tests as needed
4. Create a changeset: `pnpm changeset`
5. Commit using conventional commit format
6. Open a Pull Request

### Pre-commit Hooks

Husky runs the following checks before each commit:
- **Lint-staged**: ESLint and Prettier on staged TypeScript, `rustfmt` and `clippy` on staged Rust
- **Commitlint**: Validates commit message format
- **Type check**: `tsc` for the frontends and `cargo check` for Rust

## License

See LICENSE file for details.
