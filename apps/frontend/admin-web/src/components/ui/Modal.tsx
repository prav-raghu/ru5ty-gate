import { useEffect, useId, useRef, type ReactNode } from "react";

interface ModalProps {
    title: string;
    onClose: () => void;
    children: ReactNode;
}

export function Modal({ title, onClose, children }: ModalProps) {
    const titleId = useId();
    const panelRef = useRef<HTMLDivElement>(null);
    const onCloseRef = useRef(onClose);

    useEffect(() => {
        onCloseRef.current = onClose;
    }, [onClose]);

    useEffect(() => {
        const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        panelRef.current?.focus();
        const onKeyDown = (event: KeyboardEvent): void => {
            if (event.key === "Escape") {
                onCloseRef.current();
            }
        };
        document.addEventListener("keydown", onKeyDown);
        return () => {
            document.removeEventListener("keydown", onKeyDown);
            previous?.focus();
        };
    }, []);

    return (
        <div className="fixed inset-0 z-40 flex items-center justify-center bg-foreground/40 p-4">
            <div
                ref={panelRef}
                role="dialog"
                aria-modal="true"
                aria-labelledby={titleId}
                tabIndex={-1}
                className="w-full max-w-lg rounded-lg border border-border bg-card p-6 text-card-foreground shadow-lg focus:outline-none"
            >
                <h2 id={titleId} className="mb-4 text-lg font-semibold">
                    {title}
                </h2>
                {children}
            </div>
        </div>
    );
}
