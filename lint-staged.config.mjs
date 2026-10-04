function rustChecks() {
    return ["cargo fmt --all", "cargo clippy --workspace --all-targets --locked -- -D warnings"];
}

export default {
    "*.{ts,tsx}": ["pnpm exec prettier --write", "pnpm exec eslint --fix"],
    "*.{js,jsx,cjs,mjs}": ["pnpm exec prettier --write", "pnpm exec eslint --fix"],
    "*.{json,yml,yaml}": ["pnpm exec prettier --write"],
    "*.md": ["pnpm exec prettier --write"],
    "*.prisma": ["pnpm exec prettier --write"],
    "*.rs": rustChecks,
};
