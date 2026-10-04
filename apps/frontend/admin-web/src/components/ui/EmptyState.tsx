import type { ReactNode } from "react";

interface EmptyStateProps {
    title: string;
    description: string;
    action?: ReactNode;
}

export function EmptyState({ title, description, action }: EmptyStateProps) {
    return (
        <div className="rounded-lg border border-dashed border-border p-10 text-center">
            <h3 className="text-base font-semibold text-foreground">{title}</h3>
            <p className="mx-auto mt-1 max-w-md text-sm text-muted-foreground">{description}</p>
            {action ? <div className="mt-4 flex justify-center">{action}</div> : null}
        </div>
    );
}
