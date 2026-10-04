import { Button } from "./Button";

interface PaginationProps {
    page: number;
    hasNext: boolean;
    onPrevious: () => void;
    onNext: () => void;
}

export function Pagination({ page, hasNext, onPrevious, onNext }: PaginationProps) {
    return (
        <nav aria-label="Pagination" className="flex items-center justify-between pt-4">
            <Button variant="secondary" onClick={onPrevious} disabled={page <= 1}>
                Previous
            </Button>
            <span className="text-sm text-muted-foreground">Page {page}</span>
            <Button variant="secondary" onClick={onNext} disabled={!hasNext}>
                Next
            </Button>
        </nav>
    );
}
