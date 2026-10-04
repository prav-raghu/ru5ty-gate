export type GatewayStatus = "online" | "delayed" | "offline" | "never";

const ONLINE_WINDOW_MS = 3 * 60 * 1000;
const DELAYED_WINDOW_MS = 15 * 60 * 1000;

export function gatewayStatus(lastHeartbeatAt: string | null, now: number): GatewayStatus {
    if (lastHeartbeatAt === null) {
        return "never";
    }
    const age = now - new Date(lastHeartbeatAt).getTime();
    if (age <= ONLINE_WINDOW_MS) {
        return "online";
    }
    return age <= DELAYED_WINDOW_MS ? "delayed" : "offline";
}

export const GATEWAY_STATUS_LABELS: Record<GatewayStatus, string> = {
    online: "Online",
    delayed: "Delayed",
    offline: "Offline",
    never: "Never connected",
};
