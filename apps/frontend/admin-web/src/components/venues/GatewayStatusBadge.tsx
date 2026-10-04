import { GATEWAY_STATUS_LABELS, type GatewayStatus } from "../../utils/gateway-status";
import { cn } from "../../utils/cn";

interface GatewayStatusBadgeProps {
    status: GatewayStatus;
}

const DOT_CLASSES: Record<GatewayStatus, string> = {
    online: "bg-primary",
    delayed: "bg-muted-foreground",
    offline: "bg-destructive",
    never: "bg-border",
};

export function GatewayStatusBadge({ status }: GatewayStatusBadgeProps) {
    return (
        <span className="inline-flex items-center gap-2 text-sm">
            <span aria-hidden="true" className={cn("size-2 rounded-full", DOT_CLASSES[status])} />
            {GATEWAY_STATUS_LABELS[status]}
        </span>
    );
}
