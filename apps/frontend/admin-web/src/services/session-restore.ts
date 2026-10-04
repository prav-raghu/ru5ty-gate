import { useAuthStore } from "../store/auth.store";
import { apiClient } from "./api-client";
import { authService } from "./auth.service";

export async function restoreSession(): Promise<void> {
    const token = await apiClient.refreshAccessToken();
    if (token === null) {
        return;
    }
    try {
        const user = await authService.currentUser();
        useAuthStore.getState().setAuth(user, token);
    } catch {
        useAuthStore.getState().clearAuth();
    }
}
