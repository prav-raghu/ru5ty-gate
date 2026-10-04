const mockApiClient = { get: jest.fn(), post: jest.fn(), put: jest.fn(), delete: jest.fn() };
jest.mock("../../../src/services/api-client", () => ({ apiClient: mockApiClient }));

import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VenueDetail } from "../../../src/pages/VenueDetail";
import { useAuthStore } from "../../../src/store/auth.store";
import { useToastStore } from "../../../src/store/toast.store";
import { captiveSession, gateway, ok, venue } from "../../fixtures";
import { renderWithProviders } from "../../render-with-providers";

function signIn(role: string): void {
    const permissions = role === "Super Admin" ? ["venue:read", "venue:write"] : ["venue:read"];
    act(() => useAuthStore.getState().setAuth({ id: "u1", username: "admin", email: "admin@test.com", role, permissions }, "token"));
}

function route(): void {
    mockApiClient.get.mockImplementation(async (url: string) => {
        if (url.includes("/gateways")) {
            return ok([gateway]);
        }
        if (url.includes("/sessions")) {
            return ok([captiveSession]);
        }
        return ok(venue);
    });
}

function renderDetail() {
    return renderWithProviders(<VenueDetail />, { path: "/venues/:venueId", route: `/venues/${venue.id}` });
}

describe("VenueDetail page", () => {
    afterEach(() => {
        act(() => {
            useAuthStore.getState().clearAuth();
            useToastStore.getState().clear();
        });
    });

    it("shows a skeleton, then settings, gateways and sessions", async () => {
        signIn("Super Admin");
        route();
        renderDetail();

        expect(screen.getByRole("status", { name: "Loading venue" })).toBeInTheDocument();
        expect(await screen.findByRole("heading", { name: "Cafe Durban", level: 1 })).toBeInTheDocument();
        expect(screen.getByText("cafe-01")).toBeInTheDocument();
        expect(screen.getByText("The page the client asked for")).toBeInTheDocument();
        expect(await screen.findByText("front-desk")).toBeInTheDocument();
        expect(await screen.findByText("aa:bb:cc:dd:ee:ff")).toBeInTheDocument();
        expect(within(screen.getByRole("navigation", { name: "Breadcrumb" })).getByRole("link", { name: "Venues" })).toHaveAttribute(
            "href",
            "/venues",
        );
    });

    it("shows an error state and retries", async () => {
        signIn("Super Admin");
        let venueCalls = 0;
        mockApiClient.get.mockImplementation(async (url: string) => {
            if (url.includes("/gateways") || url.includes("/sessions")) {
                return ok([]);
            }
            venueCalls += 1;
            if (venueCalls === 1) {
                throw new Error("down");
            }
            return ok(venue);
        });
        const user = userEvent.setup();
        renderDetail();

        expect(await screen.findByText("Could not load this venue.")).toBeInTheDocument();
        await user.click(screen.getByRole("button", { name: "Try again" }));
        expect(await screen.findByRole("heading", { name: "Cafe Durban", level: 1 })).toBeInTheDocument();
    });

    it("hides edit for users without write access", async () => {
        signIn("Viewer");
        route();
        renderDetail();
        await screen.findByRole("heading", { name: "Cafe Durban", level: 1 });
        expect(screen.queryByRole("button", { name: "Edit venue" })).not.toBeInTheDocument();
    });

    it("shows inline errors when the edit form is invalid", async () => {
        signIn("Super Admin");
        route();
        const user = userEvent.setup();
        renderDetail();

        await user.click(await screen.findByRole("button", { name: "Edit venue" }));
        const dialog = screen.getByRole("dialog", { name: "Edit venue" });
        await user.clear(within(dialog).getByLabelText("Name"));
        await user.click(within(dialog).getByRole("button", { name: "Save changes" }));

        expect(await within(dialog).findByText("Name is required")).toBeInTheDocument();
        expect(mockApiClient.put).not.toHaveBeenCalled();
    });

    it("saves edits", async () => {
        signIn("Super Admin");
        route();
        mockApiClient.put.mockResolvedValue(ok({ ...venue, name: "Cafe Beachfront" }));
        const user = userEvent.setup();
        renderDetail();

        await user.click(await screen.findByRole("button", { name: "Edit venue" }));
        const dialog = screen.getByRole("dialog", { name: "Edit venue" });
        const name = within(dialog).getByLabelText("Name");
        await user.clear(name);
        await user.type(name, "Cafe Beachfront");
        await user.click(within(dialog).getByRole("button", { name: "Save changes" }));

        await waitFor(() =>
            expect(mockApiClient.put).toHaveBeenCalledWith(`/api/v1/venues/${venue.id}`, {
                name: "Cafe Beachfront",
                sessionDurationSecs: 3600,
                allowNewSessions: true,
            }),
        );
        await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
        expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "success", message: "Venue updated" });
    });

    it("clears an existing redirect URL when the field is emptied", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockImplementation(async (url: string) => {
            if (url.includes("/gateways") || url.includes("/sessions")) {
                return ok([]);
            }
            return ok({ ...venue, redirectUrl: "https://example.com/welcome" });
        });
        mockApiClient.put.mockResolvedValue(ok(venue));
        const user = userEvent.setup();
        renderDetail();

        await user.click(await screen.findByRole("button", { name: "Edit venue" }));
        const dialog = screen.getByRole("dialog", { name: "Edit venue" });
        await user.clear(within(dialog).getByLabelText("Redirect URL"));
        await user.click(within(dialog).getByRole("button", { name: "Save changes" }));

        await waitFor(() =>
            expect(mockApiClient.put).toHaveBeenCalledWith(
                `/api/v1/venues/${venue.id}`,
                expect.objectContaining({ clearRedirectUrl: true }),
            ),
        );
    });
});
