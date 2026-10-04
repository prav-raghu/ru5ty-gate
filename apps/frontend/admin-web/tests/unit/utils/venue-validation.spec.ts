import { gatewayCreateSchema, venueCreateSchema, venueUpdateSchema } from "../../../src/utils/venue-validation";

const valid = { code: "venue-1", name: "Cafe", sessionDurationSecs: 3600, redirectUrl: "", allowNewSessions: true };

describe("venueCreateSchema", () => {
    it("accepts a valid venue", () => {
        expect(venueCreateSchema.safeParse(valid).success).toBe(true);
    });

    it.each([
        ["too short code", { code: "ab" }],
        ["code with spaces", { code: "bad code" }],
        ["code over 64 characters", { code: "a".repeat(65) }],
        ["blank name", { name: "   " }],
        ["name over 120 characters", { name: "n".repeat(121) }],
        ["duration under 60", { sessionDurationSecs: 59 }],
        ["duration over 7 days", { sessionDurationSecs: 604_801 }],
        ["fractional duration", { sessionDurationSecs: 90.5 }],
        ["non http redirect", { redirectUrl: "ftp://example.com" }],
        ["malformed redirect", { redirectUrl: "not a url" }],
        ["redirect over 2048 characters", { redirectUrl: `https://example.com/${"a".repeat(2048)}` }],
    ])("rejects %s", (_label, override) => {
        expect(venueCreateSchema.safeParse({ ...valid, ...override }).success).toBe(false);
    });

    it("accepts an https redirect and an omitted redirect", () => {
        expect(venueCreateSchema.safeParse({ ...valid, redirectUrl: "https://example.com/welcome" }).success).toBe(true);
        expect(venueCreateSchema.safeParse({ ...valid, redirectUrl: undefined }).success).toBe(true);
    });

    it("reports a message for a NaN duration", () => {
        const result = venueCreateSchema.safeParse({ ...valid, sessionDurationSecs: Number.NaN });
        expect(result.success).toBe(false);
    });
});

describe("venueUpdateSchema", () => {
    it("does not require a code", () => {
        expect(venueUpdateSchema.safeParse({ name: "Cafe", sessionDurationSecs: 3600, allowNewSessions: true }).success).toBe(true);
    });
});

describe("gatewayCreateSchema", () => {
    it("accepts a name up to 80 characters", () => {
        expect(gatewayCreateSchema.safeParse({ name: "g".repeat(80) }).success).toBe(true);
    });

    it("rejects blank and over-long names", () => {
        expect(gatewayCreateSchema.safeParse({ name: " " }).success).toBe(false);
        expect(gatewayCreateSchema.safeParse({ name: "g".repeat(81) }).success).toBe(false);
    });
});
