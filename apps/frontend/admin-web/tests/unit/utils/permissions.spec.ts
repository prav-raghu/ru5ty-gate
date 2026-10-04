import { canManageVenues } from "../../../src/utils/permissions";

describe("canManageVenues", () => {
    it("allows Super Admin", () => {
        expect(canManageVenues("Super Admin")).toBe(true);
    });

    it("denies other roles and a missing role", () => {
        expect(canManageVenues("Viewer")).toBe(false);
        expect(canManageVenues(undefined)).toBe(false);
    });
});
