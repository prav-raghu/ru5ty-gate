import { apiClient } from "./api-client";
import type { ApiEnvelope, CurrentUser } from "../types/portal";

export interface LoginData {
    authToken: string;
    refreshToken: string;
    username: string;
    mfaRequired?: boolean;
    mfaToken?: string;
}

interface CurrentUserResponse {
    id: string;
    username: string;
    email: string;
    roles: { name: string };
}

function loginData(envelope: ApiEnvelope<LoginData>): LoginData {
    if (!envelope.isSuccessful || envelope.data === undefined) {
        throw new Error(envelope.message ?? "Invalid credentials");
    }
    return envelope.data;
}

export const authService = {
    async login(email: string, password: string): Promise<LoginData> {
        return loginData(await apiClient.post<ApiEnvelope<LoginData>>("/api/v1/auth/login", { email, password, rememberMe: false }));
    },
    async verifyMfa(mfaToken: string, code: string): Promise<LoginData> {
        return loginData(
            await apiClient.post<ApiEnvelope<LoginData>>("/api/v1/auth/verify-login-mfa", { mfaToken, code, rememberMe: false }),
        );
    },
    async currentUser(): Promise<CurrentUser> {
        const user = await apiClient.get<CurrentUserResponse>("/api/v1/auth/me");
        return { id: user.id, username: user.username, email: user.email, role: user.roles.name };
    },
};
