import { useState } from "react";
import { useCreateGateway, useGateways, useRotateGatewayKey } from "../../hooks/use-gateways";
import { useNow } from "../../hooks/use-now";
import type { Gateway, GatewayWithKey } from "../../types/portal";
import { formatOptionalDateTime } from "../../utils/date";
import { gatewayStatus } from "../../utils/gateway-status";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { ErrorState } from "../ui/ErrorState";
import { Modal } from "../ui/Modal";
import { TableSkeleton } from "../ui/Skeleton";
import { ApiKeyDialog } from "./ApiKeyDialog";
import { CreateGatewayForm } from "./CreateGatewayForm";
import { GatewayStatusBadge } from "./GatewayStatusBadge";

interface GatewayListProps {
    venueId: string;
    canManage: boolean;
}

const STATUS_REFRESH_MS = 30_000;

type Dialog =
    { kind: "create" } | { kind: "confirm-rotate"; gateway: Gateway } | { kind: "key"; title: string; created: GatewayWithKey } | null;

export function GatewayList({ venueId, canManage }: GatewayListProps) {
    const gateways = useGateways(venueId);
    const now = useNow(STATUS_REFRESH_MS);
    const [dialog, setDialog] = useState<Dialog>(null);

    const create = useCreateGateway(venueId, (created) => setDialog({ kind: "key", title: "Gateway registered", created }));
    const rotate = useRotateGatewayKey(venueId, (rotated) => setDialog({ kind: "key", title: "New API key issued", created: rotated }));

    const addButton = canManage ? <Button onClick={() => setDialog({ kind: "create" })}>Register gateway</Button> : null;

    return (
        <section aria-labelledby="gateways-heading" className="rounded-lg border border-border bg-card p-4">
            <div className="mb-4 flex items-center justify-between">
                <h2 id="gateways-heading" className="text-lg font-semibold">
                    Gateways
                </h2>
                {gateways.data && gateways.data.length > 0 ? addButton : null}
            </div>

            {gateways.isPending ? <TableSkeleton label="Loading gateways" rows={3} /> : null}
            {gateways.isError ? <ErrorState message="Could not load gateways." onRetry={() => void gateways.refetch()} /> : null}
            {gateways.data && gateways.data.length === 0 ? (
                <EmptyState
                    title="No gateways yet"
                    description="Register a gateway to get an API key for a router agent."
                    action={addButton}
                />
            ) : null}
            {gateways.data && gateways.data.length > 0 ? (
                <div className="overflow-x-auto">
                    <table className="w-full text-left text-sm">
                        <thead className="text-muted-foreground">
                            <tr>
                                <th scope="col" className="py-2 pr-4 font-medium">
                                    Name
                                </th>
                                <th scope="col" className="py-2 pr-4 font-medium">
                                    Status
                                </th>
                                <th scope="col" className="py-2 pr-4 font-medium">
                                    Last heartbeat
                                </th>
                                <th scope="col" className="py-2 pr-4 font-medium">
                                    Agent
                                </th>
                                <th scope="col" className="py-2 pr-4 font-medium">
                                    Sessions
                                </th>
                                <th scope="col" className="py-2 pr-4 font-medium">
                                    Queued events
                                </th>
                                {canManage ? (
                                    <th scope="col" className="py-2 font-medium">
                                        Actions
                                    </th>
                                ) : null}
                            </tr>
                        </thead>
                        <tbody>
                            {gateways.data.map((gateway) => (
                                <tr key={gateway.id} className="border-t border-border">
                                    <td className="py-3 pr-4 font-medium">{gateway.name}</td>
                                    <td className="py-3 pr-4">
                                        <GatewayStatusBadge status={gatewayStatus(gateway.lastHeartbeatAt, now)} />
                                    </td>
                                    <td className="py-3 pr-4">{formatOptionalDateTime(gateway.lastHeartbeatAt)}</td>
                                    <td className="py-3 pr-4">{gateway.agentVersion ?? "-"}</td>
                                    <td className="py-3 pr-4">{gateway.activeSessions ?? "-"}</td>
                                    <td className="py-3 pr-4">{gateway.pendingEvents ?? "-"}</td>
                                    {canManage ? (
                                        <td className="py-3">
                                            <Button variant="secondary" onClick={() => setDialog({ kind: "confirm-rotate", gateway })}>
                                                Rotate key
                                            </Button>
                                        </td>
                                    ) : null}
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            ) : null}

            {dialog?.kind === "create" ? (
                <Modal title="Register gateway" onClose={() => setDialog(null)}>
                    <CreateGatewayForm
                        submitting={create.isPending}
                        onSubmit={(name) => create.mutate(name)}
                        onCancel={() => setDialog(null)}
                    />
                </Modal>
            ) : null}
            {dialog?.kind === "confirm-rotate" ? (
                <Modal title={`Rotate key for ${dialog.gateway.name}?`} onClose={() => setDialog(null)}>
                    <p className="mb-4 text-sm text-foreground">
                        The current key stops working immediately. The router agent will lose its connection until you put the new key in
                        its configuration.
                    </p>
                    <div className="flex justify-end gap-2">
                        <Button variant="secondary" onClick={() => setDialog(null)}>
                            Cancel
                        </Button>
                        <Button variant="danger" loading={rotate.isPending} onClick={() => rotate.mutate(dialog.gateway.id)}>
                            Rotate key
                        </Button>
                    </div>
                </Modal>
            ) : null}
            {dialog?.kind === "key" ? (
                <ApiKeyDialog
                    title={dialog.title}
                    gatewayName={dialog.created.gateway.name}
                    apiKey={dialog.created.apiKey}
                    onClose={() => setDialog(null)}
                />
            ) : null}
        </section>
    );
}
