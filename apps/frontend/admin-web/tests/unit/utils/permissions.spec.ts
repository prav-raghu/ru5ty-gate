import { canManageVenues } from "../../../src/utils/permissions";

describe("canManageVenues", () => {
    it("allows users holding venue:write", () => {
        expect(canManageVenues(["venue:read", "venue:write"])).toBe(true);
    });

    it("denies read-only users, empty lists and a missing list", () => {
        expect(canManageVenues(["venue:read"])).toBe(false);
        expect(canManageVenues([])).toBe(false);
        expect(canManageVenues(undefined)).toBe(false);
    });
});
