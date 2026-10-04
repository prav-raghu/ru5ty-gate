export const ROUTES = {
    HOME: "/",
    LOGIN: "/login",
    DASHBOARD: "/dashboard",
    USERS: "/users",
    SETTINGS: "/settings",
    VENUES: "/venues",
    VENUE_DETAIL: "/venues/:venueId",
} as const;

export const venueDetailPath = (venueId: string): string => `${ROUTES.VENUES}/${venueId}`;

export type RouteKey = keyof typeof ROUTES;
export type RouteValue = (typeof ROUTES)[RouteKey];
