import { isAxiosError } from "axios";
import type { ApiEnvelope } from "../types/portal";

const FALLBACK_MESSAGE = "Something went wrong. Please try again.";

export function errorMessage(error: unknown): string {
    if (isAxiosError<ApiEnvelope<unknown>>(error)) {
        const serverMessage = error.response?.data?.message;
        if (serverMessage) {
            return serverMessage;
        }
        if (error.response?.status === 403) {
            return "You do not have permission to do that.";
        }
        if (!error.response) {
            return "Cannot reach the server. Check your connection.";
        }
    }
    return error instanceof Error && error.message ? error.message : FALLBACK_MESSAGE;
}
