import { GATEWAY_STATUS_LABELS, gatewayStatus } from "../../../src/utils/gateway-status";

const NOW = Date.parse("2026-10-04T12:00:00Z");
const ago = (minutes: number): string => new Date(NOW - minutes * 60_000).toISOString();

describe("gatewayStatus", () => {
    it("is never when there has been no heartbeat", () => {
        expect(gatewayStatus(null, NOW)).toBe("never");
    });

    it("is online up to three minutes", () => {
        expect(gatewayStatus(ago(0), NOW)).toBe("online");
        expect(gatewayStatus(ago(3), NOW)).toBe("online");
    });

    it("is delayed between three and fifteen minutes", () => {
        expect(gatewayStatus(ago(3.5), NOW)).toBe("delayed");
        expect(gatewayStatus(ago(15), NOW)).toBe("delayed");
    });

    it("is offline beyond fifteen minutes", () => {
        expect(gatewayStatus(ago(16), NOW)).toBe("offline");
    });

    it("has a label for every status", () => {
        expect(Object.keys(GATEWAY_STATUS_LABELS).sort()).toEqual(["delayed", "never", "offline", "online"]);
    });
});
