import { useState } from "react";
import { useParams } from "react-router-dom";
import { AppShell } from "../components/layout/AppShell";
import { Button } from "../components/ui/Button";
import { ErrorState } from "../components/ui/ErrorState";
import { Modal } from "../components/ui/Modal";
import { Skeleton } from "../components/ui/Skeleton";
import { GatewayList } from "../components/venues/GatewayList";
import { SessionsPanel } from "../components/venues/SessionsPanel";
import { VenueForm } from "../components/venues/VenueForm";
import { ROUTES } from "../constants/routes";
import { useUpdateVenue, useVenue } from "../hooks/use-venues";
import { useAuthStore } from "../store/auth.store";
import type { Venue } from "../types/portal";
import { formatDuration } from "../utils/date";
import { canManageVenues } from "../utils/permissions";
import type { VenueCreateForm } from "../utils/venue-validation";

interface VenueSettingsProps {
    venue: Venue;
    canManage: boolean;
}

function VenueSettings({ venue, canManage }: VenueSettingsProps) {
    const [editing, setEditing] = useState(false);
    const update = useUpdateVenue(venue.id, () => setEditing(false));

    const submit = (values: VenueCreateForm): void => {
        const redirectUrl = values.redirectUrl ?? "";
        update.mutate({
            name: values.name,
            sessionDurationSecs: values.sessionDurationSecs,
            allowNewSessions: values.allowNewSessions,
            ...(redirectUrl ? { redirectUrl } : {}),
            ...(redirectUrl === "" && venue.redirectUrl !== null ? { clearRedirectUrl: true } : {}),
        });
    };

    return (
        <section aria-labelledby="settings-heading" className="mb-6 rounded-lg border border-border bg-card p-4">
            <div className="mb-3 flex items-center justify-between">
                <h2 id="settings-heading" className="text-lg font-semibold">
                    Settings
                </h2>
                {canManage ? (
                    <Button variant="secondary" onClick={() => setEditing(true)}>
                        Edit venue
                    </Button>
                ) : null}
            </div>
            <dl className="grid gap-3 text-sm sm:grid-cols-2">
                <div>
                    <dt className="text-muted-foreground">Code</dt>
                    <dd className="font-mono">{venue.code}</dd>
                </div>
                <div>
                    <dt className="text-muted-foreground">Session length</dt>
                    <dd>{formatDuration(venue.sessionDurationSecs)}</dd>
                </div>
                <div>
                    <dt className="text-muted-foreground">New sessions</dt>
                    <dd>{venue.allowNewSessions ? "Accepting" : "Paused"}</dd>
                </div>
                <div>
                    <dt className="text-muted-foreground">Redirect URL</dt>
                    <dd className="break-all">{venue.redirectUrl ?? "The page the client asked for"}</dd>
                </div>
            </dl>
            {editing ? (
                <Modal title="Edit venue" onClose={() => setEditing(false)}>
                    <VenueForm
                        mode="edit"
                        defaultValues={{
                            code: venue.code,
                            name: venue.name,
                            sessionDurationSecs: venue.sessionDurationSecs,
                            redirectUrl: venue.redirectUrl ?? "",
                            allowNewSessions: venue.allowNewSessions,
                        }}
                        submitting={update.isPending}
                        onSubmit={submit}
                        onCancel={() => setEditing(false)}
                    />
                </Modal>
            ) : null}
        </section>
    );
}

export function VenueDetail() {
    const { venueId = "" } = useParams<{ venueId: string }>();
    const role = useAuthStore((state) => state.user?.role);
    const canManage = canManageVenues(role);
    const venue = useVenue(venueId);

    const crumbs = [{ label: "Venues", to: ROUTES.VENUES }, { label: venue.data?.name ?? "Venue" }];

    return (
        <AppShell breadcrumbs={crumbs}>
            {venue.isPending ? (
                <div role="status" aria-label="Loading venue" className="space-y-4">
                    <Skeleton className="h-8 w-64" />
                    <Skeleton className="h-32 w-full" />
                </div>
            ) : null}
            {venue.isError ? <ErrorState message="Could not load this venue." onRetry={() => void venue.refetch()} /> : null}
            {venue.data ? (
                <>
                    <h1 className="mb-6 text-2xl font-bold">{venue.data.name}</h1>
                    <VenueSettings venue={venue.data} canManage={canManage} />
                    <div className="space-y-6">
                        <GatewayList venueId={venue.data.id} canManage={canManage} />
                        <SessionsPanel venueId={venue.data.id} />
                    </div>
                </>
            ) : null}
        </AppShell>
    );
}
