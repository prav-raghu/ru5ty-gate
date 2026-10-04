import { useEffect } from "react";
import { useToastStore, type Toast } from "../../store/toast.store";
import { cn } from "../../utils/cn";

const DISMISS_AFTER_MS = 6000;

function ToastItem({ toast }: { toast: Toast }) {
    const dismiss = useToastStore((state) => state.dismiss);

    useEffect(() => {
        const timer = setTimeout(() => dismiss(toast.id), DISMISS_AFTER_MS);
        return () => clearTimeout(timer);
    }, [dismiss, toast.id]);

    return (
        <li
            className={cn(
                "flex items-start justify-between gap-3 rounded-md border px-4 py-3 text-sm shadow-md",
                toast.kind === "error" ? "border-destructive/40 bg-card text-foreground" : "border-primary/40 bg-card text-foreground",
            )}
        >
            <span>{toast.message}</span>
            <button
                type="button"
                aria-label="Dismiss notification"
                onClick={() => dismiss(toast.id)}
                className="rounded px-1 text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
                ×
            </button>
        </li>
    );
}

export function ToastViewport() {
    const toasts = useToastStore((state) => state.toasts);
    return (
        <ul aria-live="polite" className="fixed bottom-4 right-4 z-50 flex w-80 max-w-[calc(100vw-2rem)] flex-col gap-2">
            {toasts.map((toast) => (
                <ToastItem key={toast.id} toast={toast} />
            ))}
        </ul>
    );
}
