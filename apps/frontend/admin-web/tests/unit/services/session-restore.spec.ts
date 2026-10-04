const mockApiClient = { refreshAccessToken: jest.fn(), get: jest.fn() };
jest.mock("../../../src/services/api-client", () => ({ apiClient: mockApiClient }));

import { restoreSession } from "../../../src/services/session-restore";
import { authTokenStore } from "../../../src/store/auth-token.store";
import { useAuthStore } from "../../../src/store/auth.store";

const me = {
    id: "u1",
    username: "admin",
    email: "admin@test.com",
    roles: { name: "Super Admin" },
    permissions: ["venue:read", "venue:write"],
};

describe("restoreSession", () => {
    afterEach(() => {
        useAuthStore.getState().clearAuth();
    });

    it("signs the user in when the refresh cookie is still valid", async () => {
        mockApiClient.refreshAccessToken.mockResolvedValue("fresh");
        mockApiClient.get.mockResolvedValue(me);

        await restoreSession();

        expect(useAuthStore.getState()).toMatchObject({
            isAuthenticated: true,
            user: { role: "Super Admin", permissions: ["venue:read", "venue:write"] },
        });
        expect(authTokenStore.getToken()).toBe("fresh");
    });

    it("stays signed out when there is no valid cookie", async () => {
        mockApiClient.refreshAccessToken.mockResolvedValue(null);

        await restoreSession();

        expect(useAuthStore.getState().isAuthenticated).toBe(false);
        expect(mockApiClient.get).not.toHaveBeenCalled();
    });

    it("stays signed out when the profile cannot be loaded", async () => {
        mockApiClient.refreshAccessToken.mockImplementation(async () => {
            authTokenStore.setToken("fresh");
            return "fresh";
        });
        mockApiClient.get.mockRejectedValue(new Error("down"));

        await restoreSession();

        expect(useAuthStore.getState().isAuthenticated).toBe(false);
        expect(authTokenStore.getToken()).toBeNull();
    });
});
