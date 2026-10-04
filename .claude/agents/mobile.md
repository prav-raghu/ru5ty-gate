---
name: mobile
description: Use when working on mobile apps under apps/mobile/. Covers React Native with Expo — screens, components, hooks, API client, push notifications, native device features (camera, location, secure storage), offline support, navigation, and native builds with EAS for iOS/Android. Also use for new mobile screens and connecting mobile to backend APIs.
tools: Read, Edit, Write, Grep, Glob, Bash
model: inherit
---

## Expo changes every SDK — check the docs before using an Expo API

Read the major version of `expo` in `apps/mobile/customer-mobile/package.json` (currently SDK 57, React Native 0.86, React 19.2.3) and read the matching docs at `https://docs.expo.dev/versions/v<major>.0.0/` before writing code that touches an Expo or React Native API. `https://docs.expo.dev/llms.txt` is an index of all Expo docs. Install Expo-related packages with `npx expo install <package>` (run from the app folder), never `pnpm add`, so the versions match the SDK.

## App location

```
apps/mobile/
└── customer-mobile/     React Native (Expo) customer app
```

## Tech stack

| Concern | Technology |
|---------|-----------|
| UI | React Native components and `StyleSheet` |
| Tooling | Expo SDK (Metro bundler, `expo` CLI), EAS for native builds |
| Routing | Expo Router (file-based). Not installed yet: add it with the first feature that needs a second screen, following the SDK's installation guide |
| Data fetching | TanStack Query |
| API client | Axios, centralised in `src/services/apiClient.ts` |
| Token storage | `expo-secure-store` (Keychain / Keystore) |
| Other persistence | `@react-native-async-storage/async-storage` (non-sensitive cache only) |
| State | React Context or Zustand |
| Forms | React Hook Form + Zod |
| Native features | `expo-notifications`, `expo-image-picker` / `expo-camera`, `expo-location` |
| Tests | Not set up yet. When the first tests are written, add `jest-expo` and React Native Testing Library through `npx expo install` |

## Directory structure

```
apps/mobile/customer-mobile/
├── app.json                 Expo config (name, slug, bundle identifiers, icons)
├── index.ts                 registerRootComponent entry
├── assets/                  icons and splash images
└── src/
    ├── App.tsx
    ├── screens/             one folder or file per screen
    ├── components/          shared and feature components
    ├── hooks/               React Query hooks and other hooks
    ├── services/            apiClient.ts and one service file per resource
    ├── store/
    ├── utils/
    └── constants/           config.ts reads EXPO_PUBLIC_* values
```

Do not deviate from this structure. When Expo Router is added, routes live in `src/app/` and screens import from the folders above.

## Dev server and port

The Metro dev server runs on **4007** (`expo start --port 4007`), per the 4000-range rule in `CLAUDE.md`. Never change it back to Metro's default 8081.

```bash
pnpm dev:mobile                      # same as: pnpm --filter customer-mobile dev
pnpm --filter customer-mobile android
pnpm --filter customer-mobile ios
```

## API client

Same pattern as the web frontends, with tokens in secure storage and never in `AsyncStorage`:

```typescript
import axios from "axios";
import * as SecureStore from "expo-secure-store";

import { API_BASE_URL } from "../constants/config";

const apiClient = axios.create({ baseURL: API_BASE_URL, timeout: 15000 });

apiClient.interceptors.request.use(async (config) => {
    const token = await SecureStore.getItemAsync("access_token");
    if (token) config.headers.Authorization = `Bearer ${token}`;
    return config;
});

apiClient.interceptors.response.use(
    (response) => response,
    async (error) => {
        if (error.response?.status === 401) await SecureStore.deleteItemAsync("access_token");
        return Promise.reject(error);
    },
);

export default apiClient;
```

`constants/config.ts` reads the base URL with the exact static form Expo inlines at build time:

```typescript
export const API_BASE_URL = process.env.EXPO_PUBLIC_MOBILE_API_BASE_URL ?? "http://localhost:4000";
```

## Screen pattern

Every screen handles all three states (loading, error with retry, empty) and supports pull-to-refresh:

```tsx
import { ActivityIndicator, FlatList, Pressable, RefreshControl, Text, View } from "react-native";

import { useProducts } from "../hooks/useProducts";

export default function ProductsScreen() {
    const { data, isLoading, isError, isRefetching, refetch } = useProducts();

    if (isLoading) return <ActivityIndicator />;
    if (isError) {
        return (
            <View>
                <Text>Something went wrong.</Text>
                <Pressable onPress={() => refetch()}>
                    <Text>Retry</Text>
                </Pressable>
            </View>
        );
    }

    return (
        <FlatList
            data={data}
            keyExtractor={(product) => product.id}
            renderItem={({ item }) => <ProductCard product={item} />}
            ListEmptyComponent={<Text>No products found.</Text>}
            refreshControl={<RefreshControl refreshing={isRefetching} onRefresh={refetch} />}
        />
    );
}
```

## React Query hooks

Cursor-paginated lists use `useInfiniteQuery` against the API's `nextCursor` / `hasMore` shape, with `FlatList` driving the next page:

```tsx
const { data, fetchNextPage, hasNextPage } = useProducts();
const products = data?.pages.flatMap((page) => page.data) ?? [];

<FlatList
    data={products}
    onEndReached={() => hasNextPage && fetchNextPage()}
    onEndReachedThreshold={0.5}
    keyExtractor={(product) => product.id}
    renderItem={({ item }) => <ProductCard product={item} />}
/>;
```

Use `FlatList` (or `FlashList`) for every list, never `ScrollView` plus `map` for server data.

## Authentication — token storage

Access and refresh tokens go in `expo-secure-store`:

```typescript
import * as SecureStore from "expo-secure-store";

export async function saveTokens(accessToken: string, refreshToken: string): Promise<void> {
    await Promise.all([
        SecureStore.setItemAsync("access_token", accessToken),
        SecureStore.setItemAsync("refresh_token", refreshToken),
    ]);
}
```

Never put tokens in `AsyncStorage`, in `EXPO_PUBLIC_*` variables, or in the app bundle. `EXPO_PUBLIC_*` values are inlined into the compiled app in plain text.

## Push notifications, camera and location

Use `expo-notifications`, `expo-image-picker` / `expo-camera` and `expo-location`. For each module:

1. Read its page in the docs for the current SDK, including the permission flow and any `app.json` plugin or permission strings it needs
2. Check whether it works in Expo Go; if it does not, use a development build (`expo-dev-client`)
3. Request permission at the moment the feature is used, never at app start, and handle a denial without crashing

Photos from the camera are uploaded as multipart to the API (which stores them via `common/storage`). Never send raw base64 into a database column and never keep captured images only on the device.

## Offline support

```typescript
const queryClient = new QueryClient({
    defaultOptions: {
        queries: { networkMode: "offlineFirst", staleTime: 1000 * 60 * 10, gcTime: 1000 * 60 * 60 * 24 },
    },
});
```

Persist the cache for offline reads with `@tanstack/react-query-persist-client` and an `AsyncStorage` persister. Persist only non-sensitive data.

## App config and native builds

`app.json` carries the app identity. The template ships placeholder identifiers that must be replaced before any store build:

```json
{
    "expo": {
        "name": "Customer Mobile",
        "slug": "customer-mobile",
        "ios": { "bundleIdentifier": "com.ru5tygate.customer" },
        "android": { "package": "com.ru5tygate.customer" }
    }
}
```

`pnpm --filter customer-mobile build` runs `expo export`, which bundles the iOS and Android JavaScript to Hermes bytecode in `dist/`. It does not produce an installable app and is the cheap check used in CI. Installable builds use EAS (`eas build`), which needs an Expo account and signing credentials: iOS builds also need an Apple Developer account, and `expo run:ios` needs macOS.

## Environment variables

Mobile uses the `EXPO_PUBLIC_<SCOPE>_*` convention from `rules/frontend.md`, scope `MOBILE`:

```env
EXPO_PUBLIC_MOBILE_API_BASE_URL=http://localhost:4000
```

`localhost` only works where the app runs on the same machine as the API. For anything else, point at an address the device can reach:

| Where the app runs | Value |
|--------------------|-------|
| iOS simulator | `http://localhost:4000` |
| Android emulator | `http://10.0.2.2:4000` |
| Physical device on the same network | `http://<LAN IP of this machine>:4000` |

For a physical device, Windows Firewall must also allow inbound connections on 4007 (Metro) and 4000 (the gateway). Native requests do not send a browser origin, so the API's `CORS_ORIGIN` does not apply to them. Restart the dev server after changing a `.env` value.

## Critical rules

Never store tokens in `AsyncStorage` or `EXPO_PUBLIC_*` variables; use `expo-secure-store`. Never put a secret in the app: everything in the bundle is readable. Never call backend APIs outside `src/services/apiClient.ts`. Never fetch data with `useEffect` plus Axios; use React Query. Never render server lists with `ScrollView` plus `map`; use `FlatList`. Every screen implements loading, error with retry, and empty states, and data screens support pull-to-refresh. Install Expo packages with `npx expo install`. Replace the placeholder `bundleIdentifier` and `package` in `app.json` before a store build. Keep the dev server on port 4007.
