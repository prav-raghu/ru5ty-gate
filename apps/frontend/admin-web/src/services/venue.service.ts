import { apiClient } from "./api-client";
import type { ApiEnvelope, PageParams, Venue, VenueInput, VenueUpdateInput } from "../types/portal";

const BASE = "/api/v1/venues";

function unwrap<T>(envelope: ApiEnvelope<T>): T {
    if (!envelope.isSuccessful || envelope.data === undefined) {
        throw new Error(envelope.message ?? "Request failed");
    }
    return envelope.data;
}

export const venueService = {
    async list({ limit, offset }: PageParams): Promise<Venue[]> {
        return unwrap(await apiClient.get<ApiEnvelope<Venue[]>>(`${BASE}?limit=${limit}&offset=${offset}`));
    },
    async get(venueId: string): Promise<Venue> {
        return unwrap(await apiClient.get<ApiEnvelope<Venue>>(`${BASE}/${venueId}`));
    },
    async create(input: VenueInput): Promise<Venue> {
        return unwrap(await apiClient.post<ApiEnvelope<Venue>>(BASE, input));
    },
    async update(venueId: string, input: VenueUpdateInput): Promise<Venue> {
        return unwrap(await apiClient.put<ApiEnvelope<Venue>>(`${BASE}/${venueId}`, input));
    },
};
