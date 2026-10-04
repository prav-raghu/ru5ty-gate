export const QUERY_KEYS = {
    USER: "user",
    USERS: "users",
    PROFILE: "profile",
    SYSTEM_STATS: "system-stats",
    VENUES: "venues",
    VENUE: "venue",
    GATEWAYS: "gateways",
    SESSIONS: "sessions",
} as const;

export type QueryKey = (typeof QUERY_KEYS)[keyof typeof QUERY_KEYS];
