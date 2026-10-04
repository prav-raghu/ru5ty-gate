const mockApiClient = { get: jest.fn(), post: jest.fn(), put: jest.fn(), delete: jest.fn() };
jest.mock("../../../src/services/api-client", () => ({ apiClient: mockApiClient }));

import { authService } from "../../../src/services/auth.service";
import { gatewayService } from "../../../src/services/gateway.service";
import { sessionService } from "../../../src/services/session.service";
import { venueService } from "../../../src/services/venue.service";

const ok = <T>(data: T) => ({ isSuccessful: true, data });
const failed = { isSuccessful: false, message: "Nope" };

describe("venueService", () => {
    it("lists with pagination and unwraps the envelope", async () => {
        mockApiClient.get.mockResolvedValue(ok([{ id: "v1" }]));
        await expect(venueService.list({ limit: 20, offset: 40 })).resolves.toEqual([{ id: "v1" }]);
        expect(mockApiClient.get).toHaveBeenCalledWith("/api/v1/venues?limit=20&offset=40");
    });

    it("gets, creates and updates a venue", async () => {
        mockApiClient.get.mockResolvedValue(ok({ id: "v1" }));
        mockApiClient.post.mockResolvedValue(ok({ id: "v2" }));
        mockApiClient.put.mockResolvedValue(ok({ id: "v1" }));

        await venueService.get("v1");
        await venueService.create({ code: "abc", name: "A", sessionDurationSecs: 3600, allowNewSessions: true });
        await venueService.update("v1", { name: "B" });

        expect(mockApiClient.get).toHaveBeenCalledWith("/api/v1/venues/v1");
        expect(mockApiClient.post).toHaveBeenCalledWith("/api/v1/venues", expect.objectContaining({ code: "abc" }));
        expect(mockApiClient.put).toHaveBeenCalledWith("/api/v1/venues/v1", { name: "B" });
    });

    it("throws the server message when the envelope is unsuccessful", async () => {
        mockApiClient.get.mockResolvedValue(failed);
        await expect(venueService.get("v1")).rejects.toThrow("Nope");
    });

    it("throws a default message when the envelope has no data or message", async () => {
        mockApiClient.get.mockResolvedValue({ isSuccessful: true });
        await expect(venueService.get("v1")).rejects.toThrow("Request failed");
    });
});

describe("gatewayService", () => {
    it("lists, registers and rotates", async () => {
        mockApiClient.get.mockResolvedValue(ok([]));
        mockApiClient.post.mockResolvedValue(ok({ gateway: { id: "g1" }, apiKey: "g1.secret" }));

        await gatewayService.list("v1");
        const created = await gatewayService.create("v1", "front-desk");
        await gatewayService.rotateKey("g1");

        expect(mockApiClient.get).toHaveBeenCalledWith("/api/v1/venues/v1/gateways");
        expect(mockApiClient.post).toHaveBeenCalledWith("/api/v1/venues/v1/gateways", { name: "front-desk" });
        expect(mockApiClient.post).toHaveBeenCalledWith("/api/v1/gateways/g1/rotate-key");
        expect(created.apiKey).toBe("g1.secret");
    });

    it("throws on an unsuccessful envelope", async () => {
        mockApiClient.get.mockResolvedValue(failed);
        await expect(gatewayService.list("v1")).rejects.toThrow("Nope");
    });
});

describe("sessionService", () => {
    it("lists sessions with filters", async () => {
        mockApiClient.get.mockResolvedValue(ok([{ id: "s1" }]));
        await expect(sessionService.list("v1", { limit: 20, offset: 0, openOnly: true })).resolves.toEqual([{ id: "s1" }]);
        expect(mockApiClient.get).toHaveBeenCalledWith("/api/v1/venues/v1/sessions?limit=20&offset=0&openOnly=true");
    });

    it("throws on an unsuccessful envelope", async () => {
        mockApiClient.get.mockResolvedValue(failed);
        await expect(sessionService.list("v1", { limit: 20, offset: 0, openOnly: false })).rejects.toThrow("Nope");
    });
});

describe("authService", () => {
    it("logs in and returns the data", async () => {
        mockApiClient.post.mockResolvedValue(ok({ authToken: "t", refreshToken: "r", username: "admin" }));
        await expect(authService.login("a@b.com", "password1")).resolves.toMatchObject({ authToken: "t" });
        expect(mockApiClient.post).toHaveBeenCalledWith("/api/v1/auth/login", {
            email: "a@b.com",
            password: "password1",
            rememberMe: false,
        });
    });

    it("throws the server message on a failed login", async () => {
        mockApiClient.post.mockResolvedValue({ isSuccessful: false, message: "Invalid credentials" });
        await expect(authService.login("a@b.com", "password1")).rejects.toThrow("Invalid credentials");
    });

    it("falls back to a default failed-login message", async () => {
        mockApiClient.post.mockResolvedValue({ isSuccessful: false });
        await expect(authService.login("a@b.com", "password1")).rejects.toThrow("Invalid credentials");
    });

    it("verifies an MFA code", async () => {
        mockApiClient.post.mockResolvedValue(ok({ authToken: "t", refreshToken: "r", username: "admin" }));
        await authService.verifyMfa("mfa-token", "123456");
        expect(mockApiClient.post).toHaveBeenCalledWith("/api/v1/auth/verify-login-mfa", {
            mfaToken: "mfa-token",
            code: "123456",
            rememberMe: false,
        });
    });

    it("maps the current user role", async () => {
        mockApiClient.get.mockResolvedValue({
            id: "u1",
            username: "admin",
            email: "a@b.com",
            roles: { name: "Super Admin" },
            permissions: ["venue:read", "venue:write"],
        });
        await expect(authService.currentUser()).resolves.toEqual({
            id: "u1",
            username: "admin",
            email: "a@b.com",
            role: "Super Admin",
            permissions: ["venue:read", "venue:write"],
        });
    });
});
