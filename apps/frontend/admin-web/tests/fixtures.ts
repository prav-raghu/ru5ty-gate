import type { CaptiveSession, Gateway, Venue } from "../src/types/portal";

export const venue: Venue = {
    id: "11111111-1111-1111-1111-111111111111",
    code: "cafe-01",
    name: "Cafe Durban",
    sessionDurationSecs: 3600,
    redirectUrl: null,
    allowNewSessions: true,
    createdAt: "2026-10-01T08:00:00Z",
    updatedAt: "2026-10-01T08:00:00Z",
};

export const gateway: Gateway = {
    id: "22222222-2222-2222-2222-222222222222",
    venueId: venue.id,
    name: "front-desk",
    lastHeartbeatAt: null,
    agentVersion: "0.1.0",
    uptimeSecs: 7200,
    activeSessions: 4,
    pendingEvents: 0,
    lastSyncOkAt: null,
    createdAt: "2026-10-01T08:00:00Z",
};

export const captiveSession: CaptiveSession = {
    id: "33333333-3333-3333-3333-333333333333",
    venueId: venue.id,
    gatewayId: gateway.id,
    gatewayName: "front-desk",
    macIdentifier: "aa:bb:cc:dd:ee:ff",
    clientIdentifier: null,
    grantedAt: "2026-10-04T08:00:00",
    expiresAt: "2026-10-04T09:00:00",
    endedAt: null,
    endReason: null,
};

export const ok = <T>(data: T) => ({ isSuccessful: true, data });
