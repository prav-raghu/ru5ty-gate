const mockNavigate = jest.fn();

jest.mock("react-router-dom", () => ({
    useNavigate: () => mockNavigate,
}));

const mockApiClient = { get: jest.fn(), post: jest.fn() };
jest.mock("../../../src/services/api-client", () => ({
    apiClient: mockApiClient,
}));

import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Login } from "../../../src/pages/Login";
import { useAuthStore } from "../../../src/store/auth.store";
import { useToastStore } from "../../../src/store/toast.store";

const loginSuccess = { isSuccessful: true, data: { authToken: "token-123", refreshToken: "r", username: "admin" } };
const me = {
    id: "u1",
    username: "admin",
    email: "admin@test.com",
    roles: { name: "Super Admin" },
    permissions: ["venue:read", "venue:write"],
};

async function fillCredentials(email: string, password: string): Promise<void> {
    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/email/i), email);
    await user.type(screen.getByLabelText(/password/i), password);
    await user.click(screen.getByRole("button", { name: /sign in/i }));
}

describe("Login", () => {
    afterEach(() => {
        act(() => {
            useAuthStore.getState().clearAuth();
            useToastStore.getState().clear();
        });
    });

    it("shows inline validation errors when submitted empty", async () => {
        const user = userEvent.setup();
        render(<Login />);

        await user.click(screen.getByRole("button", { name: /sign in/i }));

        expect(await screen.findByText("Email is required")).toBeInTheDocument();
        expect(screen.getByText("Password must be at least 8 characters")).toBeInTheDocument();
        expect(mockApiClient.post).not.toHaveBeenCalled();
        expect(useToastStore.getState().toasts).toEqual([]);
    });

    it("shows a validation error for a malformed email", async () => {
        render(<Login />);
        await fillCredentials("not-an-email", "longenough");
        expect(await screen.findByText("Invalid email address")).toBeInTheDocument();
        expect(mockApiClient.post).not.toHaveBeenCalled();
    });

    it("logs in, loads the current user and navigates to venues", async () => {
        mockApiClient.post.mockResolvedValue(loginSuccess);
        mockApiClient.get.mockResolvedValue(me);
        render(<Login />);

        await fillCredentials("admin@test.com", "longenough");

        await waitFor(() => expect(mockNavigate).toHaveBeenCalledWith("/venues"));
        expect(useAuthStore.getState()).toMatchObject({ isAuthenticated: true, user: { role: "Super Admin" } });
    });

    it("toasts the server message on invalid credentials", async () => {
        mockApiClient.post.mockResolvedValue({ isSuccessful: false, message: "Invalid credentials" });
        render(<Login />);

        await fillCredentials("admin@test.com", "wrongpassword");

        await waitFor(() => expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "error", message: "Invalid credentials" }));
        expect(useAuthStore.getState().isAuthenticated).toBe(false);
        expect(mockNavigate).not.toHaveBeenCalled();
    });

    it("asks for an authenticator code when MFA is required, then completes login", async () => {
        mockApiClient.post
            .mockResolvedValueOnce({
                isSuccessful: true,
                data: { authToken: "", refreshToken: "", username: "admin", mfaRequired: true, mfaToken: "mfa-1" },
            })
            .mockResolvedValueOnce(loginSuccess);
        mockApiClient.get.mockResolvedValue(me);
        const user = userEvent.setup();
        render(<Login />);

        await fillCredentials("admin@test.com", "longenough");
        const code = await screen.findByLabelText("Authenticator code");
        expect(useAuthStore.getState().isAuthenticated).toBe(false);

        await user.type(code, "123456");
        await user.click(screen.getByRole("button", { name: "Verify" }));

        await waitFor(() => expect(mockNavigate).toHaveBeenCalledWith("/venues"));
        expect(mockApiClient.post).toHaveBeenLastCalledWith("/api/v1/auth/verify-login-mfa", {
            mfaToken: "mfa-1",
            code: "123456",
            rememberMe: false,
        });
    });

    it("shows an inline error for a malformed MFA code and toasts a wrong code", async () => {
        mockApiClient.post
            .mockResolvedValueOnce({
                isSuccessful: true,
                data: { authToken: "", refreshToken: "", username: "admin", mfaRequired: true, mfaToken: "mfa-1" },
            })
            .mockResolvedValueOnce({ isSuccessful: false, message: "Invalid code" });
        const user = userEvent.setup();
        render(<Login />);

        await fillCredentials("admin@test.com", "longenough");
        const code = await screen.findByLabelText("Authenticator code");

        await user.type(code, "12");
        await user.click(screen.getByRole("button", { name: "Verify" }));
        expect(await screen.findByText("Enter the 6-digit code from your authenticator app")).toBeInTheDocument();

        await user.clear(code);
        await user.type(code, "654321");
        await user.click(screen.getByRole("button", { name: "Verify" }));
        await waitFor(() => expect(useToastStore.getState().toasts[0]).toMatchObject({ kind: "error", message: "Invalid code" }));
        expect(useAuthStore.getState().isAuthenticated).toBe(false);
    });
});
