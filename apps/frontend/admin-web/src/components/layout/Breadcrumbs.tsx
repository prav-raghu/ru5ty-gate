import { Link } from "react-router-dom";

export interface Crumb {
    label: string;
    to?: string;
}

interface BreadcrumbsProps {
    items: Crumb[];
}

export function Breadcrumbs({ items }: BreadcrumbsProps) {
    return (
        <nav aria-label="Breadcrumb" className="mb-4 text-sm text-muted-foreground">
            <ol className="flex flex-wrap items-center gap-2">
                {items.map((item, index) => (
                    <li key={`${item.label}-${index}`} className="flex items-center gap-2">
                        {item.to ? (
                            <Link
                                to={item.to}
                                className="rounded hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                            >
                                {item.label}
                            </Link>
                        ) : (
                            <span aria-current="page" className="text-foreground">
                                {item.label}
                            </span>
                        )}
                        {index < items.length - 1 ? <span aria-hidden="true">/</span> : null}
                    </li>
                ))}
            </ol>
        </nav>
    );
}
