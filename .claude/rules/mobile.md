---
paths:
  - "apps/mobile/**/*.ts"
  - "apps/mobile/**/*.tsx"
---

# Mobile Rules

You are working on the React Native (Expo) mobile app under `apps/mobile/`. Expo changes every SDK: read the matching docs at `https://docs.expo.dev/versions/v<major>.0.0/` (major = the `expo` version in `package.json`) before using an Expo or React Native API. Full guidance is in `.claude/agents/mobile.md`.

## Environment variables

Mobile follows the `EXPO_PUBLIC_<SCOPE>_*` convention (see `rules/frontend.md`) with scope `MOBILE`: `EXPO_PUBLIC_MOBILE_API_BASE_URL`. Read it as `process.env.EXPO_PUBLIC_MOBILE_API_BASE_URL` exactly; Expo only inlines that static form, not bracket access or destructuring. Everything with the `EXPO_PUBLIC_` prefix is readable in the compiled app, so it never holds a secret.

## Port

The Metro dev server runs on **4007** (`expo start --port 4007`). Never use Metro's default 8081.

## Non-negotiable

- Tokens: `expo-secure-store` only — never `AsyncStorage`, never `EXPO_PUBLIC_*`
- ALL API calls: through `src/services/apiClient.ts` — never raw `fetch`/Axios in components
- ALL data fetching: React Query — never `useEffect` plus Axios
- ALL server lists: `FlatList` (or `FlashList`) with `onEndReached` for pagination — never `ScrollView` plus `map`
- ALL data screens: pull-to-refresh with `RefreshControl`, plus loading, error-with-retry and empty states
- Install Expo-related packages with `npx expo install` so versions match the SDK
- No `any` type, no comments in code

## Native features

Check the module's docs page for the current SDK before writing code: the permission flow, any `app.json` plugin or permission string it needs, and whether it works in Expo Go or needs a development build. Request permissions when the feature is used, not at startup, and handle a denial.

## Required React Query config for mobile

```typescript
const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            networkMode: "offlineFirst",
            staleTime: 1000 * 60 * 10,
            retry: 2,
            gcTime: 1000 * 60 * 60 * 24,
        },
    },
});
```

## Naming

Same as the React frontends:
- Screens: `PascalCase.tsx` in `src/screens/`
- Components: `PascalCase.tsx` in `src/components/{feature}/`
- Hooks: `use{Name}.ts` in `src/hooks/`
- Services: `camelCase.ts` in `src/services/`

## Before a native build

- [ ] `ios.bundleIdentifier` and `android.package` in `app.json` replaced with the real identifiers
- [ ] `name` and `slug` in `app.json` updated
- [ ] `EXPO_PUBLIC_MOBILE_API_BASE_URL` points at an address the device can reach, not `localhost` (Android emulator `10.0.2.2`, physical device the LAN IP or a domain)
- [ ] `pnpm --filter customer-mobile build` passes (bundles iOS and Android JavaScript)
- [ ] Permission strings for every native module are set in `app.json`
- [ ] Signing credentials configured through EAS

## Before marking complete

Run `pnpm --filter customer-mobile typecheck` — zero errors required.
