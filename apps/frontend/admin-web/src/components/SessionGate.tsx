import { useEffect, useState, type ReactNode } from "react";
import { restoreSession } from "../services/session-restore";
import { Skeleton } from "./ui/Skeleton";

interface SessionGateProps {
    children: ReactNode;
}

export function SessionGate({ children }: SessionGateProps) {
    const [restoring, setRestoring] = useState(true);

    useEffect(() => {
        let active = true;
        const done = (): void => {
            if (active) {
                setRestoring(false);
            }
        };
        void restoreSession().then(done, done);
        return () => {
            active = false;
        };
    }, []);

    if (restoring) {
        return (
            <div
                role="status"
                aria-label="Restoring your session"
                className="flex min-h-screen items-center justify-center bg-background p-8"
            >
                <Skeleton className="h-8 w-48" />
            </div>
        );
    }

    return <>{children}</>;
}
