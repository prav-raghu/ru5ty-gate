import { useEffect, useState } from "react";

export function useNow(intervalMs: number): number {
    const [now, setNow] = useState<number>(() => Date.now());

    useEffect(() => {
        const timer = setInterval(() => setNow(Date.now()), intervalMs);
        return () => clearInterval(timer);
    }, [intervalMs]);

    return now;
}
