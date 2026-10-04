const mockApiClient = { get: jest.fn(), post: jest.fn(), put: jest.fn(), delete: jest.fn(), refreshAccessToken: jest.fn() };
jest.mock("../../src/services/api-client", () => ({ apiClient: mockApiClient }));

import { act, render, screen } from "@testing-library/react";
import App from "../../src/App";
import { useAuthStore } from "../../src/store/auth.store";
import { ok, venue } from "../fixtures";

describe("App", () => {
    beforeEach(() => {
        mockApiClient.refreshAccessToken.mockResolvedValue(null);
    });

    afterEach(() => {
        act(() => {
            useAuthStore.getState().clearAuth();
        });
        window.history.pushState({}, "", "/");
    });

    it("redirects unauthenticated visitors from / to /login", async () => {
        render(<App />);
        expect(await screen.findByText("Admin Login")).toBeInTheDocument();
    });

    it("redirects unauthenticated visitors from a venue page to /login", async () => {
        window.history.pushState({}, "", "/venues");
        render(<App />);
        expect(await screen.findByText("Admin Login")).toBeInTheDocument();
    });

    it("restores the session from the refresh cookie on a page reload", async () => {
        mockApiClient.refreshAccessToken.mockResolvedValue("fresh");
        mockApiClient.get.mockImplementation(async (url: string) =>
            url === "/api/v1/auth/me"
                ? {
                      id: "u1",
                      username: "admin",
                      email: "admin@test.com",
                      roles: { name: "Super Admin" },
                      permissions: ["venue:read", "venue:write"],
                  }
                : ok([venue]),
        );
        window.history.pushState({}, "", "/venues");

        render(<App />);

        expect(screen.getByRole("status", { name: "Restoring your session" })).toBeInTheDocument();
        expect(await screen.findByText("Cafe Durban")).toBeInTheDocument();
        expect(screen.queryByText("Admin Login")).not.toBeInTheDocument();
        expect(useAuthStore.getState().user?.role).toBe("Super Admin");
    });

    it("sends authenticated visitors from / to the venues list", async () => {
        mockApiClient.get.mockResolvedValue(ok([venue]));
        act(() => {
            useAuthStore.getState().setAuth(
                {
                    id: "u1",
                    username: "admin",
                    email: "admin@test.com",
                    role: "Super Admin",
                    permissions: ["venue:read", "venue:write"],
                },
                "token",
            );
        });

        render(<App />);

        expect(await screen.findByRole("heading", { name: "Venues" })).toBeInTheDocument();
        expect(await screen.findByText("Cafe Durban")).toBeInTheDocument();
        expect(window.location.pathname).toBe("/venues");
    });

    it("renders the not-found page for an unknown route", async () => {
        window.history.pushState({}, "", "/does-not-exist");
        render(<App />);

        expect(await screen.findByText("404")).toBeInTheDocument();
    });
});
