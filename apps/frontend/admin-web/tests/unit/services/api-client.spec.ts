const mockAxiosInstance = {
    get: jest.fn(),
    post: jest.fn(),
    put: jest.fn(),
    delete: jest.fn(),
    request: jest.fn(),
    interceptors: {
        request: { use: jest.fn() },
        response: { use: jest.fn() },
    },
};

jest.mock("axios", () => ({
    __esModule: true,
    default: {
        create: jest.fn(() => mockAxiosInstance),
    },
}));

import axios from "axios";
import { authTokenStore } from "../../../src/store/auth-token.store";
import { useAuthStore } from "../../../src/store/auth.store";
import { apiClient } from "../../../src/services/api-client";

const createOptions = (axios.create as jest.Mock).mock.calls[0][0] as { withCredentials?: boolean };

describe("apiClient", () => {
    let requestInterceptor: (config: never) => never;
    let requestErrorInterceptor: (error: Error) => never;
    let responseSuccessInterceptor: (response: never) => never;
    let responseErrorInterceptor: (error: unknown) => Promise<unknown>;

    beforeAll(() => {
        [requestInterceptor, requestErrorInterceptor] = mockAxiosInstance.interceptors.request.use.mock.calls[0];
        [responseSuccessInterceptor, responseErrorInterceptor] = mockAxiosInstance.interceptors.response.use.mock.calls[0];
    });

    beforeEach(() => {
        authTokenStore.clearToken();
    });

    describe("request interceptor", () => {
        it("attaches the bearer token when one is present", () => {
            authTokenStore.setToken("token-123");
            const config = { headers: {} as Record<string, string> };

            const result = requestInterceptor(config as never);

            expect((result as typeof config).headers.Authorization).toBe("Bearer token-123");
        });

        it("leaves headers untouched when there is no token", () => {
            const config = { headers: {} as Record<string, string> };

            const result = requestInterceptor(config as never);

            expect((result as typeof config).headers.Authorization).toBeUndefined();
        });

        it("rethrows request errors", () => {
            expect(() => requestErrorInterceptor(new Error("boom"))).toThrow("boom");
        });
    });

    describe("response interceptor", () => {
        it("passes successful responses through unchanged", () => {
            const response = { data: { ok: true } };
            expect(responseSuccessInterceptor(response as never)).toBe(response);
        });

        it("refreshes once on a 401, then retries the request with a fresh token", async () => {
            mockAxiosInstance.post.mockResolvedValue({ data: { data: { accessToken: "fresh" } } });
            mockAxiosInstance.request.mockResolvedValue({ data: "retried" });
            const config = { url: "/api/v1/venues" };

            await expect(responseErrorInterceptor({ response: { status: 401 }, config })).resolves.toEqual({ data: "retried" });

            expect(mockAxiosInstance.post).toHaveBeenCalledWith("/api/v1/auth/refresh", { rememberMe: false });
            expect(authTokenStore.getToken()).toBe("fresh");
            expect(mockAxiosInstance.request).toHaveBeenCalledWith(expect.objectContaining({ url: "/api/v1/venues", _authRetried: true }));
        });

        it("clears the session and rethrows when the refresh fails", async () => {
            authTokenStore.setToken("stale");
            mockAxiosInstance.post.mockRejectedValue(new Error("expired"));
            const error = { response: { status: 401 }, config: { url: "/api/v1/venues" } };

            await expect(responseErrorInterceptor(error)).rejects.toBe(error);

            expect(authTokenStore.getToken()).toBeNull();
            expect(useAuthStore.getState().isAuthenticated).toBe(false);
            expect(mockAxiosInstance.request).not.toHaveBeenCalled();
        });

        it("does not refresh again for a request that was already retried", async () => {
            const error = { response: { status: 401 }, config: { url: "/api/v1/venues", _authRetried: true } };

            await expect(responseErrorInterceptor(error)).rejects.toBe(error);

            expect(mockAxiosInstance.post).not.toHaveBeenCalled();
        });

        it.each(["/api/v1/auth/login", "/api/v1/auth/verify-login-mfa", "/api/v1/auth/refresh"])(
            "never refreshes for the credential route %s",
            async (url) => {
                const error = { response: { status: 401 }, config: { url } };

                await expect(responseErrorInterceptor(error)).rejects.toBe(error);

                expect(mockAxiosInstance.post).not.toHaveBeenCalled();
            },
        );

        it("does not retry the refresh route on a server error", async () => {
            const error = { response: { status: 503 }, config: { url: "/api/v1/auth/refresh", _retryCount: 0 } };

            await expect(responseErrorInterceptor(error)).rejects.toBe(error);

            expect(mockAxiosInstance.request).not.toHaveBeenCalled();
        });

        it("rethrows immediately when there is no request config", async () => {
            const error = { response: { status: 500 }, config: undefined };

            await expect(responseErrorInterceptor(error)).rejects.toBe(error);
        });

        it("retries retryable errors up to the max retry count", async () => {
            jest.useFakeTimers();
            mockAxiosInstance.request.mockResolvedValue({ data: "recovered" });
            const error = { response: { status: 503 }, config: { _retryCount: 0 } };

            const promise = responseErrorInterceptor(error);
            await jest.advanceTimersByTimeAsync(1000);

            await expect(promise).resolves.toEqual({ data: "recovered" });
            expect(mockAxiosInstance.request).toHaveBeenCalledWith(expect.objectContaining({ _retryCount: 1 }));
            jest.useRealTimers();
        });

        it("gives up and rethrows once the retry limit is exceeded", async () => {
            const error = { response: { status: 503 }, config: { _retryCount: 3 } };

            await expect(responseErrorInterceptor(error)).rejects.toBe(error);
            expect(mockAxiosInstance.request).not.toHaveBeenCalled();
        });

        it("does not retry non-retryable client errors", async () => {
            const error = { response: { status: 400 }, config: { _retryCount: 0 } };

            await expect(responseErrorInterceptor(error)).rejects.toBe(error);
            expect(mockAxiosInstance.request).not.toHaveBeenCalled();
        });
    });

    describe("refreshAccessToken", () => {
        afterEach(() => {
            Reflect.deleteProperty(navigator, "locks");
        });

        it("sends credentials with every request", () => {
            expect(createOptions.withCredentials).toBe(true);
        });

        it("exchanges the cookie for an access token and stores it", async () => {
            mockAxiosInstance.post.mockResolvedValue({ data: { data: { accessToken: "abc" } } });

            await expect(apiClient.refreshAccessToken()).resolves.toBe("abc");

            expect(authTokenStore.getToken()).toBe("abc");
        });

        it("returns null when the response has no token", async () => {
            mockAxiosInstance.post.mockResolvedValue({ data: {} });

            await expect(apiClient.refreshAccessToken()).resolves.toBeNull();
            expect(authTokenStore.getToken()).toBeNull();
        });

        it("returns null when the exchange fails", async () => {
            mockAxiosInstance.post.mockRejectedValue(new Error("401"));

            await expect(apiClient.refreshAccessToken()).resolves.toBeNull();
        });

        it("shares one in-flight exchange between concurrent callers", async () => {
            mockAxiosInstance.post.mockResolvedValue({ data: { data: { accessToken: "abc" } } });

            const [first, second] = await Promise.all([apiClient.refreshAccessToken(), apiClient.refreshAccessToken()]);

            expect(first).toBe("abc");
            expect(second).toBe("abc");
            expect(mockAxiosInstance.post).toHaveBeenCalledTimes(1);
        });

        it("starts a new exchange once the previous one has finished", async () => {
            mockAxiosInstance.post.mockResolvedValue({ data: { data: { accessToken: "abc" } } });

            await apiClient.refreshAccessToken();
            await apiClient.refreshAccessToken();

            expect(mockAxiosInstance.post).toHaveBeenCalledTimes(2);
        });

        it("serialises across tabs with a Web Lock when the browser has them", async () => {
            const request = jest.fn((_name: string, callback: () => Promise<string | null>) => callback());
            Object.defineProperty(navigator, "locks", { value: { request }, configurable: true });
            mockAxiosInstance.post.mockResolvedValue({ data: { data: { accessToken: "abc" } } });

            await apiClient.refreshAccessToken();

            expect(request).toHaveBeenCalledWith("ru5ty-gate-refresh", expect.any(Function));
        });
    });

    describe("HTTP methods", () => {
        it("get returns the response data", async () => {
            mockAxiosInstance.get.mockResolvedValue({ data: { id: 1 } });

            await expect(apiClient.get("/things")).resolves.toEqual({ id: 1 });
            expect(mockAxiosInstance.get).toHaveBeenCalledWith("/things");
        });

        it("post returns the response data", async () => {
            mockAxiosInstance.post.mockResolvedValue({ data: { created: true } });

            await expect(apiClient.post("/things", { name: "a" })).resolves.toEqual({ created: true });
            expect(mockAxiosInstance.post).toHaveBeenCalledWith("/things", { name: "a" });
        });

        it("put returns the response data", async () => {
            mockAxiosInstance.put.mockResolvedValue({ data: { updated: true } });

            await expect(apiClient.put("/things/1", { name: "b" })).resolves.toEqual({ updated: true });
        });

        it("delete returns the response data", async () => {
            mockAxiosInstance.delete.mockResolvedValue({ data: { deleted: true } });

            await expect(apiClient.delete("/things/1")).resolves.toEqual({ deleted: true });
        });

        it("getAxiosInstance returns the underlying axios instance", () => {
            expect(apiClient.getAxiosInstance()).toBe(mockAxiosInstance);
        });
    });
});
