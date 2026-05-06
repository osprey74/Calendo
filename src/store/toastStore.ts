import { create } from "zustand";

export type ToastKind = "success" | "error" | "info";

export type Toast = {
  id: number;
  kind: ToastKind;
  message: string;
};

type State = {
  toasts: Toast[];
};

type Actions = {
  show: (message: string, kind?: ToastKind, durationMs?: number) => number;
  dismiss: (id: number) => void;
};

let nextId = 1;

export const useToastStore = create<State & Actions>((set, get) => ({
  toasts: [],

  show: (message, kind = "info", durationMs = 4000) => {
    const id = nextId++;
    set((state) => ({ toasts: [...state.toasts, { id, kind, message }] }));
    if (durationMs > 0) {
      setTimeout(() => get().dismiss(id), durationMs);
    }
    return id;
  },

  dismiss: (id) => {
    set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
  },
}));

/** Imperative shorthand usable from non-React code (e.g. store actions). */
export const toast = {
  success: (msg: string) => useToastStore.getState().show(msg, "success"),
  error: (msg: string) => useToastStore.getState().show(msg, "error", 6000),
  info: (msg: string) => useToastStore.getState().show(msg, "info"),
};
