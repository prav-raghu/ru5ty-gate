const mockApiClient = { get: jest.fn(), post: jest.fn(), put: jest.fn(), delete: jest.fn() };
jest.mock("../../../src/services/api-client", () => ({ apiClient: mockApiClient }));

import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Venues } from "../../../src/pages/Venues";
import { useAuthStore } from "../../../src/store/auth.store";
import { useToastStore } from "../../../src/store/toast.store";
import { ok, venue } from "../../fixtures";
import { renderWithProviders } from "../../render-with-providers";

function signIn(role: string): void {
    act(() => useAuthStore.getState().setAuth({ id: "u1", username: "admin", email: "admin@test.com", role }, "token"));
}

describe("Venues page", () => {
    afterEach(() => {
        act(() => {
            useAuthStore.getState().clearAuth();
            useToastStore.getState().clear();
        });
    });

    it("shows a skeleton and then the venues", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockResolvedValue(ok([venue]));
        renderWithProviders(<Venues />);

        expect(screen.getByRole("status", { name: "Loading venues" })).toBeInTheDocument();
        const link = await screen.findByRole("link", { name: "Cafe Durban" });
        expect(link).toHaveAttribute("href", `/venues/${venue.id}`);
        expect(screen.getByText("cafe-01")).toBeInTheDocument();
        expect(screen.getByText("1h 0m")).toBeInTheDocument();
        expect(screen.getByText("Accepting")).toBeInTheDocument();
    });

    it("shows an empty state with a create action", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockResolvedValue(ok([]));
        renderWithProviders(<Venues />);

        expect(await screen.findByText("No venues yet")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Create venue" })).toBeInTheDocument();
    });

    it("shows an error state and retries", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockRejectedValueOnce(new Error("down")).mockResolvedValueOnce(ok([venue]));
        const user = userEvent.setup();
        renderWithProviders(<Venues />);

        expect(await screen.findByText("Could not load venues.")).toBeInTheDocument();
        await user.click(screen.getByRole("button", { name: "Try again" }));
        expect(await screen.findByText("Cafe Durban")).toBeInTheDocument();
    });

    it("filters by name or code on the current page", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockResolvedValue(ok([venue, { ...venue, id: "other", code: "bar-02", name: "Bar Umhlanga" }]));
        const user = userEvent.setup();
        renderWithProviders(<Venues />);
        await screen.findByText("Cafe Durban");

        await user.type(screen.getByLabelText("Search this page"), "bar-02");

        expect(screen.queryByText("Cafe Durban")).not.toBeInTheDocument();
        expect(screen.getByText("Bar Umhlanga")).toBeInTheDocument();

        await user.clear(screen.getByLabelText("Search this page"));
        await user.type(screen.getByLabelText("Search this page"), "nothing-like-this");
        expect(screen.getByText("No matching venues")).toBeInTheDocument();
    });

    it("hides create for users without write access", async () => {
        signIn("Viewer");
        mockApiClient.get.mockResolvedValue(ok([venue]));
        renderWithProviders(<Venues />);
        await screen.findByText("Cafe Durban");
        expect(screen.queryByRole("button", { name: "Create venue" })).not.toBeInTheDocument();
    });

    it("shows inline validation errors without calling the API", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockResolvedValue(ok([venue]));
        const user = userEvent.setup();
        renderWithProviders(<Venues />);

        await user.click(await screen.findByRole("button", { name: "Create venue" }));
        const dialog = screen.getByRole("dialog", { name: "Create venue" });
        await user.click(within(dialog).getByRole("button", { name: "Create venue" }));

        expect(await within(dialog).findByText("Code must be at least 3 characters")).toBeInTheDocument();
        expect(within(dialog).getByText("Name is required")).toBeInTheDocument();
        expect(mockApiClient.post).not.toHaveBeenCalled();
        expect(useToastStore.getState().toasts).toEqual([]);
    });

    it("creates a venue and shows a success toast", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockResolvedValue(ok([venue]));
        mockApiClient.post.mockResolvedValue(ok({ ...venue, id: "new", code: "new-venue", name: "New Venue" }));
        const user = userEvent.setup();
        renderWithProviders(<Venues />);

        await user.click(await screen.findByRole("button", { name: "Create venue" }));
        const dialog = screen.getByRole("dialog", { name: "Create venue" });
        await user.type(within(dialog).getByLabelText("Code"), "new-venue");
        await user.type(within(dialog).getByLabelText("Name"), "New Venue");
        await user.type(within(dialog).getByLabelText("Redirect URL"), "https://example.com/welcome");
        await user.click(within(dialog).getByRole("button", { name: "Create venue" }));

        await waitFor(() =>
            expect(mockApiClient.post).toHaveBeenCalledWith("/api/v1/venues", {
                code: "new-venue",
                name: "New Venue",
                sessionDurationSecs: 3600,
                allowNewSessions: true,
                redirectUrl: "https://example.com/welcome",
            }),
        );
        await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
        expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "success", message: "Venue created" });
    });

    it("toasts the server message when creation fails", async () => {
        signIn("Super Admin");
        mockApiClient.get.mockResolvedValue(ok([venue]));
        mockApiClient.post.mockRejectedValue(new Error("Venue code already exists"));
        const user = userEvent.setup();
        renderWithProviders(<Venues />);

        await user.click(await screen.findByRole("button", { name: "Create venue" }));
        const dialog = screen.getByRole("dialog", { name: "Create venue" });
        await user.type(within(dialog).getByLabelText("Code"), "cafe-01");
        await user.type(within(dialog).getByLabelText("Name"), "Dupe");
        await user.click(within(dialog).getByRole("button", { name: "Create venue" }));

        await waitFor(() =>
            expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "error", message: "Venue code already exists" }),
        );
        expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    it("pages forward and back", async () => {
        signIn("Super Admin");
        const full = Array.from({ length: 20 }, (_, index) => ({
            ...venue,
            id: `v-${index}`,
            code: `code-${index}`,
            name: `Venue ${index}`,
        }));
        mockApiClient.get.mockResolvedValue(ok(full));
        const user = userEvent.setup();
        renderWithProviders(<Venues />);
        await screen.findByText("Venue 0");

        await user.click(screen.getByRole("button", { name: "Next" }));
        await waitFor(() => expect(mockApiClient.get).toHaveBeenLastCalledWith("/api/v1/venues?limit=20&offset=20"));
        expect(await screen.findByText("Page 2")).toBeInTheDocument();

        await user.click(screen.getByRole("button", { name: "Previous" }));
        await waitFor(() => expect(mockApiClient.get).toHaveBeenLastCalledWith("/api/v1/venues?limit=20&offset=0"));
    });
});
