import { useState } from "react";
import { Link } from "react-router-dom";
import { AppShell } from "../components/layout/AppShell";
import { Button } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { ErrorState } from "../components/ui/ErrorState";
import { Modal } from "../components/ui/Modal";
import { Pagination } from "../components/ui/Pagination";
import { TableSkeleton } from "../components/ui/Skeleton";
import { TextField } from "../components/ui/TextField";
import { VenueForm } from "../components/venues/VenueForm";
import { venueDetailPath } from "../constants/routes";
import { useCreateVenue, useVenues } from "../hooks/use-venues";
import { useAuthStore } from "../store/auth.store";
import { formatDuration } from "../utils/date";
import { canManageVenues } from "../utils/permissions";
import type { VenueCreateForm } from "../utils/venue-validation";

const PAGE_SIZE = 20;

const NEW_VENUE_DEFAULTS: VenueCreateForm = {
    code: "",
    name: "",
    sessionDurationSecs: 3600,
    redirectUrl: "",
    allowNewSessions: true,
};

export function Venues() {
    const permissions = useAuthStore((state) => state.user?.permissions);
    const canManage = canManageVenues(permissions);
    const [page, setPage] = useState(1);
    const [search, setSearch] = useState("");
    const [creating, setCreating] = useState(false);

    const venues = useVenues({ limit: PAGE_SIZE, offset: (page - 1) * PAGE_SIZE });
    const create = useCreateVenue(() => setCreating(false));

    const term = search.trim().toLowerCase();
    const visible = (venues.data ?? []).filter(
        (venue) => term === "" || venue.name.toLowerCase().includes(term) || venue.code.toLowerCase().includes(term),
    );

    const submit = (values: VenueCreateForm): void => {
        create.mutate({
            code: values.code,
            name: values.name,
            sessionDurationSecs: values.sessionDurationSecs,
            allowNewSessions: values.allowNewSessions,
            ...(values.redirectUrl ? { redirectUrl: values.redirectUrl } : {}),
        });
    };

    const createButton = canManage ? <Button onClick={() => setCreating(true)}>Create venue</Button> : null;

    return (
        <AppShell breadcrumbs={[{ label: "Venues" }]}>
            <div className="mb-6 flex flex-wrap items-center justify-between gap-4">
                <h1 className="text-2xl font-bold">Venues</h1>
                {venues.data && venues.data.length > 0 ? createButton : null}
            </div>

            <div className="mb-4 max-w-xs">
                <TextField
                    label="Search this page"
                    type="search"
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                    hint="Venue name or code"
                />
            </div>

            {venues.isPending ? <TableSkeleton label="Loading venues" /> : null}
            {venues.isError ? <ErrorState message="Could not load venues." onRetry={() => void venues.refetch()} /> : null}
            {venues.data && visible.length === 0 ? (
                <EmptyState
                    title={term ? "No matching venues" : "No venues yet"}
                    description={
                        term
                            ? "Nothing on this page matches your search."
                            : "A venue is a location with one or more routers. Create one to start registering gateways."
                    }
                    action={term ? null : createButton}
                />
            ) : null}
            {visible.length > 0 ? (
                <div className="overflow-x-auto rounded-lg border border-border bg-card">
                    <table className="w-full text-left text-sm">
                        <thead className="text-muted-foreground">
                            <tr>
                                <th scope="col" className="px-4 py-3 font-medium">
                                    Name
                                </th>
                                <th scope="col" className="px-4 py-3 font-medium">
                                    Code
                                </th>
                                <th scope="col" className="px-4 py-3 font-medium">
                                    Session length
                                </th>
                                <th scope="col" className="px-4 py-3 font-medium">
                                    New sessions
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {visible.map((venue) => (
                                <tr key={venue.id} className="border-t border-border">
                                    <td className="px-4 py-3">
                                        <Link
                                            to={venueDetailPath(venue.id)}
                                            className="font-medium text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                        >
                                            {venue.name}
                                        </Link>
                                    </td>
                                    <td className="px-4 py-3 font-mono text-xs">{venue.code}</td>
                                    <td className="px-4 py-3">{formatDuration(venue.sessionDurationSecs)}</td>
                                    <td className="px-4 py-3">{venue.allowNewSessions ? "Accepting" : "Paused"}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            ) : null}
            <Pagination
                page={page}
                hasNext={(venues.data?.length ?? 0) === PAGE_SIZE}
                onPrevious={() => setPage((current) => Math.max(1, current - 1))}
                onNext={() => setPage((current) => current + 1)}
            />

            {creating ? (
                <Modal title="Create venue" onClose={() => setCreating(false)}>
                    <VenueForm
                        mode="create"
                        defaultValues={NEW_VENUE_DEFAULTS}
                        submitting={create.isPending}
                        onSubmit={submit}
                        onCancel={() => setCreating(false)}
                    />
                </Modal>
            ) : null}
        </AppShell>
    );
}
