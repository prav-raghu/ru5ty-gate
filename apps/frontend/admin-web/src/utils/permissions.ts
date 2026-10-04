const VENUE_WRITE_ROLES: readonly string[] = ["Super Admin"];

export function canManageVenues(role: string | undefined): boolean {
    return role !== undefined && VENUE_WRITE_ROLES.includes(role);
}
