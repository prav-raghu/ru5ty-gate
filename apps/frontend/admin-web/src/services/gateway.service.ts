import { apiClient } from "./api-client";
import type { ApiEnvelope, Gateway, GatewayWithKey } from "../types/portal";

function unwrap<T>(envelope: ApiEnvelope<T>): T {
    if (!envelope.isSuccessful || envelope.data === undefined) {
        throw new Error(envelope.message ?? "Request failed");
    }
    return envelope.data;
}

export const gatewayService = {
    async list(venueId: string): Promise<Gateway[]> {
        return unwrap(await apiClient.get<ApiEnvelope<Gateway[]>>(`/api/v1/venues/${venueId}/gateways`));
    },
    async create(venueId: string, name: string): Promise<GatewayWithKey> {
        return unwrap(await apiClient.post<ApiEnvelope<GatewayWithKey>>(`/api/v1/venues/${venueId}/gateways`, { name }));
    },
    async rotateKey(gatewayId: string): Promise<GatewayWithKey> {
        return unwrap(await apiClient.post<ApiEnvelope<GatewayWithKey>>(`/api/v1/gateways/${gatewayId}/rotate-key`));
    },
};
