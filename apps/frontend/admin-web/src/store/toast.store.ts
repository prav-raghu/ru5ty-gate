import { create } from "zustand";

export type ToastKind = "success" | "error";

export interface Toast {
    id: number;
    kind: ToastKind;
    message: string;
}

interface ToastState {
    toasts: Toast[];
    push: (kind: ToastKind, message: string) => void;
    dismiss: (id: number) => void;
    clear: () => void;
}

let nextId = 1;

export const useToastStore = create<ToastState>()((set) => ({
    toasts: [],
    push: (kind, message) => {
        const id = nextId++;
        set((state) => ({ toasts: [...state.toasts, { id, kind, message }] }));
    },
    dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
    clear: () => set({ toasts: [] }),
}));

export const toast = {
    success: (message: string): void => useToastStore.getState().push("success", message),
    error: (message: string): void => useToastStore.getState().push("error", message),
};
