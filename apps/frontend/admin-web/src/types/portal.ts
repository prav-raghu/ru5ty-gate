export interface ApiEnvelope<T> {
    isSuccessful: boolean;
    data?: T;
    message?: string;
    errors?: { field: string; message: string }[];
}

export interface Venue {
    id: string;
    code: string;
    name: string;
    sessionDurationSecs: number;
    redirectUrl: string | null;
    allowNewSessions: boolean;
    createdAt: string;
    updatedAt: string;
}

export interface Gateway {
    id: string;
    venueId: string;
    name: string;
    lastHeartbeatAt: string | null;
    agentVersion: string | null;
    uptimeSecs: number | null;
    activeSessions: number | null;
    pendingEvents: number | null;
    lastSyncOkAt: string | null;
    createdAt: string;
}

export interface GatewayWithKey {
    gateway: Gateway;
    apiKey: string;
}

export interface CaptiveSession {
    id: string;
    venueId: string;
    gatewayId: string;
    macIdentifier: string;
    clientIdentifier: string | null;
    gatewayName: string | null;
    grantedAt: string;
    expiresAt: string;
    endedAt: string | null;
    endReason: string | null;
}

export interface VenueInput {
    code: string;
    name: string;
    sessionDurationSecs: number;
    redirectUrl?: string;
    allowNewSessions: boolean;
}

export interface VenueUpdateInput {
    name: string;
    sessionDurationSecs: number;
    redirectUrl?: string;
    clearRedirectUrl?: boolean;
    allowNewSessions: boolean;
}

export interface PageParams {
    limit: number;
    offset: number;
}

export interface SessionParams extends PageParams {
    openOnly: boolean;
}

export interface CurrentUser {
    id: string;
    username: string;
    email: string;
    role: string;
}
