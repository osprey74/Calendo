import { useState } from "react";
import { ConnectionPanelContent } from "./ConnectionPanel";
import { DisplayPanelContent } from "./DisplayPanel";
import "./ConnectionPanel.css";
import "./SettingsModal.css";

type TabId = "connections" | "display";

const TABS: { id: TabId; label: string }[] = [
  { id: "connections", label: "アカウント接続" },
  { id: "display", label: "表示設定" },
];

export function SettingsModal({ onClose }: { onClose: () => void }) {
  const [active, setActive] = useState<TabId>("connections");

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal settings-modal"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <header className="modal-head">
          <h2>設定</h2>
          <button type="button" className="modal-close" onClick={onClose} aria-label="閉じる">
            ×
          </button>
        </header>

        <nav className="settings-tabs" role="tablist">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={active === t.id}
              className={active === t.id ? "settings-tab active" : "settings-tab"}
              onClick={() => setActive(t.id)}
            >
              {t.label}
            </button>
          ))}
        </nav>

        <div className="settings-tabpanel">
          {active === "connections" && <ConnectionPanelContent />}
          {active === "display" && <DisplayPanelContent />}
        </div>
      </div>
    </div>
  );
}
