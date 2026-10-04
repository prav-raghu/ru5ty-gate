export const VENUE_WRITE = "venue:write";

export function canManageVenues(permissions: readonly string[] | undefined): boolean {
    return permissions?.includes(VENUE_WRITE) ?? false;
}
