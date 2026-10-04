import type { InputHTMLAttributes, Ref } from "react";
import { cn } from "../../utils/cn";

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
    label: string;
    error?: string;
    hint?: string;
    ref?: Ref<HTMLInputElement>;
}

export function TextField({ label, error, hint, id, className, ref, ...rest }: TextFieldProps) {
    const fieldId = id ?? `field-${label.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
    const describedBy = error ? `${fieldId}-error` : hint ? `${fieldId}-hint` : undefined;
    return (
        <div className="space-y-1">
            <label htmlFor={fieldId} className="block text-sm font-medium text-foreground">
                {label}
            </label>
            <input
                id={fieldId}
                ref={ref}
                aria-invalid={error ? true : undefined}
                aria-describedby={describedBy}
                className={cn(
                    "block w-full rounded-md border bg-background px-3 py-2 text-sm text-foreground",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    error ? "border-destructive" : "border-input",
                    className,
                )}
                {...rest}
            />
            {error ? (
                <p id={`${fieldId}-error`} className="text-sm text-destructive">
                    {error}
                </p>
            ) : hint ? (
                <p id={`${fieldId}-hint`} className="text-xs text-muted-foreground">
                    {hint}
                </p>
            ) : null}
        </div>
    );
}
