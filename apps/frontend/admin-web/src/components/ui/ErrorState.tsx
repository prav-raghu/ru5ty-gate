import { Button } from "./Button";

interface ErrorStateProps {
    message: string;
    onRetry: () => void;
}

export function ErrorState({ message, onRetry }: ErrorStateProps) {
    return (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/10 p-6 text-center">
            <p className="mb-4 text-sm text-foreground">{message}</p>
            <Button variant="secondary" onClick={onRetry}>
                Try again
            </Button>
        </div>
    );
}
