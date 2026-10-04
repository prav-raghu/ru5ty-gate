const mockApiClient = { get: jest.fn(), post: jest.fn(), put: jest.fn(), delete: jest.fn() };
jest.mock("../../../src/services/api-client", () => ({ apiClient: mockApiClient }));

import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ApiKeyDialog } from "../../../src/components/venues/ApiKeyDialog";
import { CreateGatewayForm } from "../../../src/components/venues/CreateGatewayForm";
import { GatewayList } from "../../../src/components/venues/GatewayList";
import { GatewayStatusBadge } from "../../../src/components/venues/GatewayStatusBadge";
import { SessionsPanel } from "../../../src/components/venues/SessionsPanel";
import { VenueForm } from "../../../src/components/venues/VenueForm";
import { toast, useToastStore } from "../../../src/store/toast.store";
import { captiveSession, gateway, ok, venue } from "../../fixtures";
import { renderWithProviders } from "../../render-with-providers";

afterEach(() => {
    act(() => useToastStore.getState().clear());
});

describe("GatewayStatusBadge", () => {
    it.each([
        ["online", "Online"],
        ["delayed", "Delayed"],
        ["offline", "Offline"],
        ["never", "Never connected"],
    ] as const)("labels %s", (status, label) => {
        render(<GatewayStatusBadge status={status} />);
        expect(screen.getByText(label)).toBeInTheDocument();
    });
});

describe("ApiKeyDialog", () => {
    it("shows the key once and copies it", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined);
        const user = userEvent.setup();
        Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
        const onClose = jest.fn();
        render(<ApiKeyDialog title="Gateway registered" gatewayName="front-desk" apiKey="g1.secret" onClose={onClose} />);

        expect(screen.getByLabelText("API key")).toHaveTextContent("g1.secret");
        await user.click(screen.getByRole("button", { name: "Copy key" }));
        expect(writeText).toHaveBeenCalledWith("g1.secret");
        expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "success" });

        await user.click(screen.getByRole("button", { name: "I have stored the key" }));
        expect(onClose).toHaveBeenCalled();
    });

    it("toasts an error when the clipboard is unavailable", async () => {
        const user = userEvent.setup();
        Object.defineProperty(navigator, "clipboard", {
            value: { writeText: jest.fn().mockRejectedValue(new Error("denied")) },
            configurable: true,
        });
        render(<ApiKeyDialog title="Key" gatewayName="g" apiKey="k" onClose={jest.fn()} />);

        await user.click(screen.getByRole("button", { name: "Copy key" }));

        await waitFor(() => expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "error" }));
    });
});

describe("CreateGatewayForm", () => {
    it("shows an inline error and does not submit a blank name", async () => {
        const onSubmit = jest.fn();
        render(<CreateGatewayForm submitting={false} onSubmit={onSubmit} onCancel={jest.fn()} />);

        await userEvent.setup().click(screen.getByRole("button", { name: "Register gateway" }));

        expect(await screen.findByText("Name is required")).toBeInTheDocument();
        expect(onSubmit).not.toHaveBeenCalled();
        expect(useToastStore.getState().toasts).toEqual([]);
    });

    it("submits the trimmed name", async () => {
        const onSubmit = jest.fn();
        const user = userEvent.setup();
        render(<CreateGatewayForm submitting={false} onSubmit={onSubmit} onCancel={jest.fn()} />);

        await user.type(screen.getByLabelText("Gateway name"), "  front-desk ");
        await user.click(screen.getByRole("button", { name: "Register gateway" }));

        await waitFor(() => expect(onSubmit).toHaveBeenCalledWith("front-desk"));
    });

    it("cancels", async () => {
        const onCancel = jest.fn();
        render(<CreateGatewayForm submitting={false} onSubmit={jest.fn()} onCancel={onCancel} />);
        await userEvent.setup().click(screen.getByRole("button", { name: "Cancel" }));
        expect(onCancel).toHaveBeenCalled();
    });
});

describe("VenueForm", () => {
    const defaults = { code: "", name: "", sessionDurationSecs: 3600, redirectUrl: "", allowNewSessions: true };

    it("shows inline errors and does not submit invalid input", async () => {
        const onSubmit = jest.fn();
        const user = userEvent.setup();
        render(<VenueForm mode="create" defaultValues={defaults} submitting={false} onSubmit={onSubmit} onCancel={jest.fn()} />);

        await user.type(screen.getByLabelText("Code"), "a b");
        await user.type(screen.getByLabelText("Redirect URL"), "ftp://x");
        await user.click(screen.getByRole("button", { name: "Create venue" }));

        expect(await screen.findByText("Use letters, digits, '-' or '_' only")).toBeInTheDocument();
        expect(screen.getByText("Name is required")).toBeInTheDocument();
        expect(screen.getByText("Enter a valid http or https URL")).toBeInTheDocument();
        expect(onSubmit).not.toHaveBeenCalled();
    });

    it("submits valid values", async () => {
        const onSubmit = jest.fn();
        const user = userEvent.setup();
        render(<VenueForm mode="create" defaultValues={defaults} submitting={false} onSubmit={onSubmit} onCancel={jest.fn()} />);

        await user.type(screen.getByLabelText("Code"), "cafe-01");
        await user.type(screen.getByLabelText("Name"), "Cafe");
        await user.click(screen.getByRole("button", { name: "Create venue" }));

        await waitFor(() => expect(onSubmit).toHaveBeenCalled());
        expect(onSubmit.mock.calls[0][0]).toEqual({
            code: "cafe-01",
            name: "Cafe",
            sessionDurationSecs: 3600,
            redirectUrl: "",
            allowNewSessions: true,
        });
    });

    it("hides the code field when editing", () => {
        render(
            <VenueForm
                mode="edit"
                defaultValues={{ ...defaults, code: "cafe-01", name: "Cafe" }}
                submitting={false}
                onSubmit={jest.fn()}
                onCancel={jest.fn()}
            />,
        );
        expect(screen.queryByLabelText("Code")).not.toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Save changes" })).toBeInTheDocument();
    });
});

describe("GatewayList", () => {
    it("shows a skeleton, then rows with status and last heartbeat", async () => {
        const heartbeat = new Date(Date.now() - 60_000).toISOString();
        mockApiClient.get.mockResolvedValue(ok([{ ...gateway, lastHeartbeatAt: heartbeat }]));

        renderWithProviders(<GatewayList venueId={venue.id} canManage />);

        expect(screen.getByRole("status", { name: "Loading gateways" })).toBeInTheDocument();
        expect(await screen.findByText("front-desk")).toBeInTheDocument();
        expect(screen.getByText("Online")).toBeInTheDocument();
        expect(screen.getByText("0.1.0")).toBeInTheDocument();
    });

    it("shows an empty state with a register action", async () => {
        mockApiClient.get.mockResolvedValue(ok([]));
        renderWithProviders(<GatewayList venueId={venue.id} canManage />);
        expect(await screen.findByText("No gateways yet")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Register gateway" })).toBeInTheDocument();
    });

    it("hides write actions for read-only users", async () => {
        mockApiClient.get.mockResolvedValue(ok([gateway]));
        renderWithProviders(<GatewayList venueId={venue.id} canManage={false} />);
        await screen.findByText("front-desk");
        expect(screen.queryByRole("button", { name: /rotate key/i })).not.toBeInTheDocument();
        expect(screen.queryByRole("button", { name: "Register gateway" })).not.toBeInTheDocument();
    });

    it("shows an error state and retries", async () => {
        mockApiClient.get.mockRejectedValueOnce(new Error("down")).mockResolvedValueOnce(ok([gateway]));
        const user = userEvent.setup();
        renderWithProviders(<GatewayList venueId={venue.id} canManage />);

        expect(await screen.findByText("Could not load gateways.")).toBeInTheDocument();
        await user.click(screen.getByRole("button", { name: "Try again" }));
        expect(await screen.findByText("front-desk")).toBeInTheDocument();
    });

    it("registers a gateway and reveals the key once", async () => {
        mockApiClient.get.mockResolvedValue(ok([]));
        mockApiClient.post.mockResolvedValue(ok({ gateway, apiKey: "g1.supersecret" }));
        const user = userEvent.setup();
        renderWithProviders(<GatewayList venueId={venue.id} canManage />);

        await user.click(await screen.findByRole("button", { name: "Register gateway" }));
        await user.type(screen.getByLabelText("Gateway name"), "front-desk");
        await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Register gateway" }));

        expect(await screen.findByLabelText("API key")).toHaveTextContent("g1.supersecret");
        expect(mockApiClient.post).toHaveBeenCalledWith(`/api/v1/venues/${venue.id}/gateways`, { name: "front-desk" });

        await user.click(screen.getByRole("button", { name: "I have stored the key" }));
        expect(screen.queryByLabelText("API key")).not.toBeInTheDocument();
    });

    it("confirms before rotating a key, then reveals the new key", async () => {
        mockApiClient.get.mockResolvedValue(ok([gateway]));
        mockApiClient.post.mockResolvedValue(ok({ gateway, apiKey: "g1.rotated" }));
        const user = userEvent.setup();
        renderWithProviders(<GatewayList venueId={venue.id} canManage />);

        await user.click(await screen.findByRole("button", { name: "Rotate key" }));
        expect(screen.getByText(/current key stops working immediately/i)).toBeInTheDocument();
        expect(mockApiClient.post).not.toHaveBeenCalled();

        await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Rotate key" }));

        expect(await screen.findByLabelText("API key")).toHaveTextContent("g1.rotated");
        expect(mockApiClient.post).toHaveBeenCalledWith(`/api/v1/gateways/${gateway.id}/rotate-key`);
    });

    it("toasts a server error when registering fails", async () => {
        mockApiClient.get.mockResolvedValue(ok([]));
        mockApiClient.post.mockRejectedValue(new Error("Gateway limit reached"));
        const user = userEvent.setup();
        renderWithProviders(<GatewayList venueId={venue.id} canManage />);

        await user.click(await screen.findByRole("button", { name: "Register gateway" }));
        await user.type(screen.getByLabelText("Gateway name"), "front-desk");
        await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Register gateway" }));

        await waitFor(() => expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "error", message: "Gateway limit reached" }));
        expect(screen.queryByLabelText("API key")).not.toBeInTheDocument();
    });
});

describe("SessionsPanel", () => {
    it("lists sessions with an open state", async () => {
        mockApiClient.get.mockResolvedValue(ok([captiveSession]));
        renderWithProviders(<SessionsPanel venueId={venue.id} />);

        expect(screen.getByRole("status", { name: "Loading sessions" })).toBeInTheDocument();
        expect(await screen.findByText("aa:bb:cc:dd:ee:ff")).toBeInTheDocument();
        expect(screen.getByText("04/10/2026 08:00:00")).toBeInTheDocument();
        expect(screen.getByText("Open")).toBeInTheDocument();
    });

    it("shows the end reason for closed sessions", async () => {
        mockApiClient.get.mockResolvedValue(ok([{ ...captiveSession, endedAt: "2026-10-04T08:30:00", endReason: "logout" }]));
        renderWithProviders(<SessionsPanel venueId={venue.id} />);
        expect(await screen.findByText("04/10/2026 08:30:00 (logout)")).toBeInTheDocument();
    });

    it("filters on the page and reports no matches", async () => {
        mockApiClient.get.mockResolvedValue(ok([captiveSession]));
        const user = userEvent.setup();
        renderWithProviders(<SessionsPanel venueId={venue.id} />);
        await screen.findByText("aa:bb:cc:dd:ee:ff");

        await user.type(screen.getByLabelText("Search this page"), "zzz");

        expect(screen.getByText("No matching sessions")).toBeInTheDocument();
    });

    it("requests open sessions only when toggled", async () => {
        mockApiClient.get.mockResolvedValue(ok([]));
        const user = userEvent.setup();
        renderWithProviders(<SessionsPanel venueId={venue.id} />);
        await screen.findByText("No sessions yet");

        await user.click(screen.getByLabelText("Open sessions only"));

        await waitFor(() => expect(mockApiClient.get).toHaveBeenLastCalledWith(expect.stringContaining("openOnly=true")));
    });

    it("shows an error state", async () => {
        mockApiClient.get.mockRejectedValue(new Error("down"));
        renderWithProviders(<SessionsPanel venueId={venue.id} />);
        expect(await screen.findByText("Could not load sessions.")).toBeInTheDocument();
    });

    it("pages forward when a full page is returned", async () => {
        const full = Array.from({ length: 20 }, (_, index) => ({ ...captiveSession, id: `s-${index}`, macIdentifier: `mac-${index}` }));
        mockApiClient.get.mockResolvedValue(ok(full));
        const user = userEvent.setup();
        renderWithProviders(<SessionsPanel venueId={venue.id} />);
        await screen.findByText("mac-0");

        await user.click(screen.getByRole("button", { name: "Next" }));

        await waitFor(() => expect(mockApiClient.get).toHaveBeenLastCalledWith(expect.stringContaining("offset=20")));
        expect(await screen.findByText("Page 2")).toBeInTheDocument();
    });
});

describe("toast helper", () => {
    it("is wired for components", () => {
        toast.success("x");
        expect(useToastStore.getState().toasts).toHaveLength(1);
    });
});
