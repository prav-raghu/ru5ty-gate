import { format, parseISO } from "date-fns";

export function formatDateTime(value: string): string {
    return format(parseISO(value), "dd/MM/yyyy HH:mm:ss");
}

export function formatOptionalDateTime(value: string | null): string {
    return value === null ? "Never" : formatDateTime(value);
}

export function formatDuration(totalSeconds: number): string {
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    if (hours >= 24) {
        const days = Math.floor(hours / 24);
        return `${days}d ${hours % 24}h`;
    }
    if (hours > 0) {
        return `${hours}h ${minutes}m`;
    }
    return minutes > 0 ? `${minutes}m` : `${totalSeconds}s`;
}
