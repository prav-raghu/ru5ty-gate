# Strapi CMS — pinned to Node 22 LTS.
# Strapi 5 supports Active/Maintenance LTS only and does not support Node 26.
# This image is intentionally independent of the pnpm workspace (the CMS is
# npm-managed) so it can run on Node 22 while the rest of the repo uses Node 26.
FROM node:22-alpine AS base
# Native deps (better-sqlite3, sharp) need a build toolchain on alpine.
RUN apk add --no-cache python3 make g++ libc6-compat vips-dev

FROM base AS builder
WORKDIR /app
ENV NODE_ENV=production
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM base AS runner
WORKDIR /app
ENV NODE_ENV=production
COPY package.json package-lock.json ./
RUN npm ci --omit=dev
COPY --from=builder /app/build ./build
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/config ./config
COPY --from=builder /app/src ./src
COPY --from=builder /app/public ./public
COPY --from=builder /app/favicon.png ./favicon.png
COPY --from=builder /app/tsconfig.json ./tsconfig.json
EXPOSE 4006
CMD ["npm", "run", "start"]
