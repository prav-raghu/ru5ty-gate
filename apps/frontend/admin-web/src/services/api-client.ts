import axios, { type AxiosInstance, type AxiosError, type InternalAxiosRequestConfig } from "axios";
import { authTokenStore } from "../store/auth-token.store";
import { useAuthStore } from "../store/auth.store";

const API_BASE_URL = import.meta.env.VITE_ADMIN_API_BASE_URL || "http://localhost:4001";
const REFRESH_URL = "/api/v1/auth/refresh";
const CREDENTIAL_URLS: readonly string[] = ["/api/v1/auth/login", "/api/v1/auth/verify-login-mfa", REFRESH_URL];
const REFRESH_LOCK = "ru5ty-gate-refresh";
const MAX_RETRIES = 3;
const RETRY_DELAY_BASE = 1000;

interface RetryConfig extends InternalAxiosRequestConfig {
    _retryCount?: number;
    _authRetried?: boolean;
}

interface RefreshEnvelope {
    data?: { accessToken?: string };
}

function isCredentialRoute(url: string | undefined): boolean {
    return url !== undefined && CREDENTIAL_URLS.includes(url);
}

class ApiClient {
    private readonly client: AxiosInstance;
    private refreshing: Promise<string | null> | null = null;

    constructor() {
        this.client = axios.create({
            baseURL: API_BASE_URL,
            timeout: 10000,
            withCredentials: true,
            headers: {
                "Content-Type": "application/json",
            },
        });

        this.setupInterceptors();
    }

    private isRetryableError(error: AxiosError): boolean {
        if (!error.response) {
            return true;
        }
        const status = error.response.status;
        return status === 408 || status === 429 || status >= 500;
    }

    private async delay(ms: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    private setupInterceptors(): void {
        this.client.interceptors.request.use(
            (config) => {
                const token = authTokenStore.getToken();
                if (token) {
                    config.headers.Authorization = `Bearer ${token}`;
                }
                return config;
            },
            (error: Error) => {
                throw error;
            },
        );

        this.client.interceptors.response.use(
            (response) => response,
            async (error: AxiosError) => {
                const config = error.config as RetryConfig;

                if (!config) {
                    throw error;
                }

                config._retryCount = config._retryCount ?? 0;

                if (error.response?.status === 401) {
                    if (isCredentialRoute(config.url) || config._authRetried) {
                        throw error;
                    }
                    const token = await this.refreshAccessToken();
                    if (token === null) {
                        useAuthStore.getState().clearAuth();
                        throw error;
                    }
                    config._authRetried = true;
                    return this.client.request(config);
                }

                if (config.url !== REFRESH_URL && config._retryCount < MAX_RETRIES && this.isRetryableError(error)) {
                    config._retryCount += 1;
                    const delayMs = RETRY_DELAY_BASE * Math.pow(2, config._retryCount - 1);
                    await this.delay(delayMs);
                    return this.client.request(config);
                }

                throw error;
            },
        );
    }

    private async exchangeRefreshCookie(): Promise<string | null> {
        try {
            const response = await this.client.post<RefreshEnvelope>(REFRESH_URL, { rememberMe: false });
            const token = response.data.data?.accessToken ?? null;
            if (token) {
                authTokenStore.setToken(token);
            }
            return token;
        } catch {
            return null;
        }
    }

    public refreshAccessToken(): Promise<string | null> {
        this.refreshing ??= (
            typeof navigator !== "undefined" && navigator.locks
                ? navigator.locks.request(REFRESH_LOCK, () => this.exchangeRefreshCookie())
                : this.exchangeRefreshCookie()
        ).finally(() => {
            this.refreshing = null;
        });
        return this.refreshing;
    }

    public async get<T>(url: string): Promise<T> {
        const response = await this.client.get<T>(url);
        return response.data;
    }

    public async post<T>(url: string, data?: unknown): Promise<T> {
        const response = await this.client.post<T>(url, data);
        return response.data;
    }

    public async put<T>(url: string, data?: unknown): Promise<T> {
        const response = await this.client.put<T>(url, data);
        return response.data;
    }

    public async delete<T>(url: string): Promise<T> {
        const response = await this.client.delete<T>(url);
        return response.data;
    }

    public getAxiosInstance(): AxiosInstance {
        return this.client;
    }
}

export const apiClient = new ApiClient();
