const mockApiClient = { get: jest.fn(), post: jest.fn(), put: jest.fn(), delete: jest.fn() };
jest.mock("../../src/services/api-client", () => ({ apiClient: mockApiClient }));

import { act, render, screen } from "@testing-library/react";
import App from "../../src/App";
import { useAuthStore } from "../../src/store/auth.store";
import { ok, venue } from "../fixtures";

describe("App", () => {
    afterEach(() => {
        act(() => {
            useAuthStore.getState().clearAuth();
        });
        window.history.pushState({}, "", "/");
    });

    it("redirects unauthenticated visitors from / to /login", () => {
        render(<App />);
        expect(screen.getByText("Admin Login")).toBeInTheDocument();
    });

    it("redirects unauthenticated visitors from a venue page to /login", () => {
        window.history.pushState({}, "", "/venues");
        render(<App />);
        expect(screen.getByText("Admin Login")).toBeInTheDocument();
    });

    it("sends authenticated visitors from / to the venues list", async () => {
        mockApiClient.get.mockResolvedValue(ok([venue]));
        act(() => {
            useAuthStore.getState().setAuth({ id: "u1", username: "admin", email: "admin@test.com", role: "Super Admin" }, "token");
        });

        render(<App />);

        expect(await screen.findByRole("heading", { name: "Venues" })).toBeInTheDocument();
        expect(await screen.findByText("Cafe Durban")).toBeInTheDocument();
        expect(window.location.pathname).toBe("/venues");
    });

    it("renders the not-found page for an unknown route", () => {
        window.history.pushState({}, "", "/does-not-exist");
        render(<App />);

        expect(screen.getByText("404")).toBeInTheDocument();
    });
});
