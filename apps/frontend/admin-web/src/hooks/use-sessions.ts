import { useQuery } from "@tanstack/react-query";
import { QUERY_KEYS } from "../constants/query-keys";
import { sessionService } from "../services/session.service";
import type { SessionParams } from "../types/portal";

export function useSessions(venueId: string, params: SessionParams) {
    return useQuery({
        queryKey: [QUERY_KEYS.SESSIONS, venueId, params],
        queryFn: () => sessionService.list(venueId, params),
    });
}
