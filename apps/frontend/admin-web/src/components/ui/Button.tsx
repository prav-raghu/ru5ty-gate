import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../utils/cn";

type ButtonVariant = "primary" | "secondary" | "danger";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
    variant?: ButtonVariant;
    loading?: boolean;
}

const VARIANTS: Record<ButtonVariant, string> = {
    primary: "bg-primary text-primary-foreground hover:bg-primary/90",
    secondary: "bg-secondary text-secondary-foreground border border-border hover:bg-accent",
    danger: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
};

export function Button({ variant = "primary", loading = false, disabled, className, children, type = "button", ...rest }: ButtonProps) {
    return (
        <button
            type={type}
            disabled={disabled || loading}
            aria-busy={loading}
            className={cn(
                "inline-flex min-h-11 md:min-h-9 items-center justify-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                "disabled:cursor-not-allowed disabled:opacity-60",
                VARIANTS[variant],
                className,
            )}
            {...rest}
        >
            {loading ? "Working…" : children}
        </button>
    );
}
