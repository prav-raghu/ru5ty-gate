import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { QUERY_KEYS } from "../constants/query-keys";
import { gatewayService } from "../services/gateway.service";
import { toast } from "../store/toast.store";
import type { GatewayWithKey } from "../types/portal";
import { errorMessage } from "../utils/api-error";

export function useGateways(venueId: string) {
    return useQuery({
        queryKey: [QUERY_KEYS.GATEWAYS, venueId],
        queryFn: () => gatewayService.list(venueId),
    });
}

export function useCreateGateway(venueId: string, onCreated: (created: GatewayWithKey) => void) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (name: string) => gatewayService.create(venueId, name),
        onSuccess: async (created) => {
            await queryClient.invalidateQueries({ queryKey: [QUERY_KEYS.GATEWAYS, venueId] });
            onCreated(created);
        },
        onError: (error: unknown) => toast.error(errorMessage(error)),
    });
}

export function useRotateGatewayKey(venueId: string, onRotated: (rotated: GatewayWithKey) => void) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (gatewayId: string) => gatewayService.rotateKey(gatewayId),
        onSuccess: async (rotated) => {
            await queryClient.invalidateQueries({ queryKey: [QUERY_KEYS.GATEWAYS, venueId] });
            onRotated(rotated);
        },
        onError: (error: unknown) => toast.error(errorMessage(error)),
    });
}
