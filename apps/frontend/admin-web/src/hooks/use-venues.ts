import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { QUERY_KEYS } from "../constants/query-keys";
import { venueService } from "../services/venue.service";
import { toast } from "../store/toast.store";
import type { PageParams, VenueInput, VenueUpdateInput } from "../types/portal";
import { errorMessage } from "../utils/api-error";

export function useVenues(params: PageParams) {
    return useQuery({
        queryKey: [QUERY_KEYS.VENUES, params],
        queryFn: () => venueService.list(params),
    });
}

export function useVenue(venueId: string) {
    return useQuery({
        queryKey: [QUERY_KEYS.VENUE, venueId],
        queryFn: () => venueService.get(venueId),
    });
}

export function useCreateVenue(onCreated: () => void) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (input: VenueInput) => venueService.create(input),
        onSuccess: async () => {
            await queryClient.invalidateQueries({ queryKey: [QUERY_KEYS.VENUES] });
            toast.success("Venue created");
            onCreated();
        },
        onError: (error: unknown) => toast.error(errorMessage(error)),
    });
}

export function useUpdateVenue(venueId: string, onUpdated: () => void) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (input: VenueUpdateInput) => venueService.update(venueId, input),
        onSuccess: async () => {
            await queryClient.invalidateQueries({ queryKey: [QUERY_KEYS.VENUE, venueId] });
            await queryClient.invalidateQueries({ queryKey: [QUERY_KEYS.VENUES] });
            toast.success("Venue updated");
            onUpdated();
        },
        onError: (error: unknown) => toast.error(errorMessage(error)),
    });
}
