import { useState } from "react";
import { useSessions } from "../../hooks/use-sessions";
import { formatDateTime } from "../../utils/date";
import { EmptyState } from "../ui/EmptyState";
import { ErrorState } from "../ui/ErrorState";
import { Pagination } from "../ui/Pagination";
import { TableSkeleton } from "../ui/Skeleton";
import { TextField } from "../ui/TextField";

interface SessionsPanelProps {
    venueId: string;
}

const PAGE_SIZE = 20;

export function SessionsPanel({ venueId }: SessionsPanelProps) {
    const [page, setPage] = useState(1);
    const [openOnly, setOpenOnly] = useState(false);
    const [search, setSearch] = useState("");
    const sessions = useSessions(venueId, { limit: PAGE_SIZE, offset: (page - 1) * PAGE_SIZE, openOnly });

    const term = search.trim().toLowerCase();
    const visible = (sessions.data ?? []).filter(
        (session) =>
            term === "" ||
            session.macIdentifier.toLowerCase().includes(term) ||
            (session.gatewayName ?? "").toLowerCase().includes(term) ||
            (session.clientIdentifier ?? "").toLowerCase().includes(term),
    );

    return (
        <section aria-labelledby="sessions-heading" className="rounded-lg border border-border bg-card p-4">
            <h2 id="sessions-heading" className="mb-4 text-lg font-semibold">
                Sessions
            </h2>
            <div className="mb-4 flex flex-wrap items-end gap-4">
                <div className="w-full max-w-xs">
                    <TextField
                        label="Search this page"
                        type="search"
                        value={search}
                        onChange={(event) => setSearch(event.target.value)}
                        hint="Client, gateway or address"
                    />
                </div>
                <label className="flex items-center gap-2 pb-2 text-sm">
                    <input
                        type="checkbox"
                        className="size-4 accent-primary"
                        checked={openOnly}
                        onChange={(event) => {
                            setOpenOnly(event.target.checked);
                            setPage(1);
                        }}
                    />
                    Open sessions only
                </label>
            </div>

            {sessions.isPending ? <TableSkeleton label="Loading sessions" /> : null}
            {sessions.isError ? <ErrorState message="Could not load sessions." onRetry={() => void sessions.refetch()} /> : null}
            {sessions.data && visible.length === 0 ? (
                <EmptyState
                    title={term ? "No matching sessions" : "No sessions yet"}
                    description={
                        term
                            ? "Nothing on this page matches your search."
                            : "Sessions appear here once a router agent reports clients connecting."
                    }
                />
            ) : null}
            {visible.length > 0 ? (
                <div className="overflow-x-auto">
                    <table className="w-full text-left text-sm">
                        <thead className="text-muted-foreground">
                            <tr>
                                <th className="py-2 pr-4 font-medium">Client</th>
                                <th className="py-2 pr-4 font-medium">Gateway</th>
                                <th className="py-2 pr-4 font-medium">Connected</th>
                                <th className="py-2 pr-4 font-medium">Expires</th>
                                <th className="py-2 font-medium">Ended</th>
                            </tr>
                        </thead>
                        <tbody>
                            {visible.map((session) => (
                                <tr key={session.id} className="border-t border-border">
                                    <td className="max-w-48 truncate py-3 pr-4 font-mono text-xs" title={session.macIdentifier}>
                                        {session.macIdentifier}
                                    </td>
                                    <td className="py-3 pr-4">{session.gatewayName ?? "-"}</td>
                                    <td className="py-3 pr-4">{formatDateTime(session.grantedAt)}</td>
                                    <td className="py-3 pr-4">{formatDateTime(session.expiresAt)}</td>
                                    <td className="py-3">
                                        {session.endedAt
                                            ? `${formatDateTime(session.endedAt)} (${session.endReason ?? "unknown"})`
                                            : "Open"}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            ) : null}
            <Pagination
                page={page}
                hasNext={(sessions.data?.length ?? 0) === PAGE_SIZE}
                onPrevious={() => setPage((current) => Math.max(1, current - 1))}
                onNext={() => setPage((current) => current + 1)}
            />
        </section>
    );
}
