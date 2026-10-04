import { apiClient } from "./api-client";
import type { ApiEnvelope, CaptiveSession, SessionParams } from "../types/portal";

export const sessionService = {
    async list(venueId: string, { limit, offset, openOnly }: SessionParams): Promise<CaptiveSession[]> {
        const envelope = await apiClient.get<ApiEnvelope<CaptiveSession[]>>(
            `/api/v1/venues/${venueId}/sessions?limit=${limit}&offset=${offset}&openOnly=${openOnly}`,
        );
        if (!envelope.isSuccessful || envelope.data === undefined) {
            throw new Error(envelope.message ?? "Request failed");
        }
        return envelope.data;
    },
};
