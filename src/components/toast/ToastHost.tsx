import { useToastStore } from "../../store/toastStore";
import "./ToastHost.css";

export function ToastHost() {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);

  if (toasts.length === 0) return null;

  return (
    <div className="toast-host" aria-live="polite" aria-atomic="false">
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast-${t.kind}`} role="status">
          <span className="toast-message">{t.message}</span>
          <button
            type="button"
            className="toast-dismiss"
            onClick={() => dismiss(t.id)}
            aria-label="閉じる"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
