import { formatDateTime, formatDuration, formatOptionalDateTime } from "../../../src/utils/date";

describe("formatDateTime", () => {
    it("renders dd/MM/yyyy HH:mm:ss", () => {
        expect(formatDateTime("2026-10-04T08:05:09")).toBe("04/10/2026 08:05:09");
    });
});

describe("formatOptionalDateTime", () => {
    it("renders Never for null", () => {
        expect(formatOptionalDateTime(null)).toBe("Never");
    });

    it("formats a present value", () => {
        expect(formatOptionalDateTime("2026-10-04T08:05:09")).toBe("04/10/2026 08:05:09");
    });
});

describe("formatDuration", () => {
    it.each([
        [45, "45s"],
        [60, "1m"],
        [1800, "30m"],
        [3600, "1h 0m"],
        [5400, "1h 30m"],
        [86_400, "1d 0h"],
        [90_000, "1d 1h"],
        [604_800, "7d 0h"],
    ])("formats %i seconds as %s", (seconds, expected) => {
        expect(formatDuration(seconds)).toBe(expected);
    });
});
