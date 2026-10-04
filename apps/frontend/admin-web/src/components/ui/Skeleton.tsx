import { cn } from "../../utils/cn";

interface SkeletonProps {
    className?: string;
}

export function Skeleton({ className }: SkeletonProps) {
    return <div aria-hidden="true" className={cn("motion-safe:animate-pulse rounded-md bg-muted", className)} />;
}

interface TableSkeletonProps {
    rows?: number;
    label: string;
}

export function TableSkeleton({ rows = 5, label }: TableSkeletonProps) {
    return (
        <div role="status" aria-label={label} className="space-y-2">
            {Array.from({ length: rows }, (_, index) => (
                <Skeleton key={index} className="h-10 w-full" />
            ))}
        </div>
    );
}
