import "@fontsource/geist-sans/400.css";
import "@fontsource/geist-sans/600.css";
import "@fontsource/geist-mono/400.css";
import "@fontsource/playfair-display/400.css";
import "@fontsource/playfair-display/400-italic.css";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";
export interface Release {
  id: string;
  version: string;
  notes: string;
  size: number;
  published_at: string;
}
export interface UpdateStatus {
  product: string;
  current_version: string;
  edition: string;
  update: Release | null;
}
export interface UpdateProps {
  productName: string;
  beforeInstall?: () => Promise<void> | void;
  blockedReason?: string;
}
type Phase =
  | "idle"
  | "checking"
  | "available"
  | "current"
  | "downloading"
  | "ready"
  | "installing";
const call = <T,>(command: string) =>
  invoke<T>(`plugin:ulix-update|${command}`);
export const openUpdateWindow = () => call<void>("open_window");
function Panel({
  productName,
  beforeInstall,
  blockedReason,
  onClose,
  onBusy,
}: UpdateProps & { onClose?: () => void; onBusy?: (value: boolean) => void }) {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState("");
  const [progress, setProgress] = useState({ downloaded: 0, total: 0 });
  const running = useRef(false);
  const alive = useRef(true);
  const started = useRef(false);
  const busy = ["checking", "downloading", "installing"].includes(phase);
  useEffect(() => {
    onBusy?.(busy);
  }, [busy, onBusy]);
  useEffect(() => {
    alive.current = true;
    let disposed = false;
    let stop: (() => void) | undefined;
    void listen<{ downloaded: number; total: number }>(
      "ulix-update-progress",
      (e) => {
        if (!disposed) setProgress(e.payload);
      },
    )
      .then((fn) => {
        if (disposed) fn();
        else stop = fn;
      })
      .catch(() => {});
    return () => {
      alive.current = false;
      disposed = true;
      stop?.();
    };
  }, []);
  const check = async () => {
    if (running.current) return;
    running.current = true;
    setPhase("checking");
    setError("");
    try {
      const value = await call<UpdateStatus>("check");
      if (alive.current) {
        setStatus(value);
        setPhase(value.update ? "available" : "current");
      }
    } catch (e) {
      if (alive.current) {
        setError(String(e));
        setPhase("idle");
      }
    } finally {
      running.current = false;
    }
  };
  useEffect(() => {
    if (!started.current) {
      started.current = true;
      void check();
    }
  }, []);
  const download = async () => {
    if (running.current) return;
    running.current = true;
    setError("");
    setProgress({ downloaded: 0, total: 0 });
    setPhase("downloading");
    try {
      await call("download");
      if (alive.current) setPhase("ready");
    } catch (e) {
      if (alive.current) {
        setError(String(e));
        setPhase("available");
      }
    } finally {
      running.current = false;
    }
  };
  const install = async () => {
    if (running.current) return;
    running.current = true;
    setError("");
    setPhase("installing");
    try {
      await beforeInstall?.();
      await call("install");
    } catch (e) {
      if (alive.current) {
        setError(String(e));
        setPhase("ready");
      }
    } finally {
      running.current = false;
    }
  };
  return (
    <section
      className="ulix-update-panel"
      aria-label={`${productName} software updates`}
    >
      <header>
        <span className="ulix-update-label">ULIX / SOFTWARE UPDATES</span>
        {onClose && (
          <button aria-label="Close updates" onClick={onClose} disabled={busy}>
            ×
          </button>
        )}
      </header>
      <div className="ulix-update-content">
        <h2>
          {productName}
          <br />
          <em>Stay current.</em>
        </h2>
        <p className="ulix-update-label">
          {status
            ? `VERSION ${status.current_version} / ${status.edition.toUpperCase()}`
            : "RELEASE SERVICE"}
        </p>
        <div aria-live="polite">
          {phase === "checking" && <p>Checking for updates…</p>}
          {phase === "current" && (
            <p>You’re running the latest available version.</p>
          )}
          {status?.update && phase !== "checking" && (
            <>
              <h3>Version {status.update.version}</h3>
              <p className="ulix-update-notes">
                {status.update.notes || "A new release is available."}
              </p>
              <p className="ulix-update-label">
                {(status.update.size / 1048576).toFixed(1)} MB · VERIFIED
                RELEASE
              </p>
            </>
          )}
          {phase === "downloading" && (
            <>
              <progress value={progress.downloaded} max={progress.total || 1} />
              <p>
                Downloading ·{" "}
                {progress.total
                  ? Math.floor((progress.downloaded / progress.total) * 100)
                  : 0}
                %
              </p>
            </>
          )}
          {phase === "ready" && (
            <p>Download verified. Installation will restart {productName}.</p>
          )}
          {phase === "installing" && (
            <p>Installing. {productName} will restart…</p>
          )}
        </div>
        {blockedReason && <p className="ulix-update-notice">{blockedReason}</p>}
        {error && (
          <p className="ulix-update-error" role="alert">
            {error}
          </p>
        )}
      </div>
      <footer>
        {phase === "available" && (
          <button
            className="ulix-update-primary"
            onClick={() => void download()}
          >
            Download update <span>↓</span>
          </button>
        )}
        {phase === "ready" && (
          <button
            className="ulix-update-primary"
            disabled={!!blockedReason}
            onClick={() => void install()}
          >
            Install and restart <span>↗</span>
          </button>
        )}
        <button onClick={() => void check()} disabled={busy}>
          {phase === "idle" || phase === "current"
            ? "Check for updates"
            : "Check again"}
          <span>↻</span>
        </button>
      </footer>
    </section>
  );
}
export function UpdatePanel(props: UpdateProps) {
  return <Panel {...props} />;
}
export function UpdateButton(props: UpdateProps & { className?: string }) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(busy);
  busyRef.current = busy;
  const dialog = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const previous =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    dialog.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busyRef.current) {
        event.preventDefault();
        setOpen(false);
      }
      if (event.key === "Tab") {
        const buttons = Array.from(
          dialog.current?.querySelectorAll<HTMLButtonElement>(
            "button:not(:disabled)",
          ) ?? [],
        );
        const first = buttons[0],
          last = buttons[buttons.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    };
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("keydown", key);
      previous?.focus();
    };
  }, [open]);
  return (
    <>
      <button
        className={props.className ?? "ulix-update-trigger"}
        onClick={() => setOpen(true)}
      >
        Updates <span aria-hidden="true">↗</span>
      </button>
      {open &&
        createPortal(
          <div className="ulix-update-overlay">
            <div
              ref={dialog}
              role="dialog"
              aria-modal="true"
              aria-label="Software updates"
            >
              <Panel
                {...props}
                onBusy={setBusy}
                onClose={() => {
                  if (!busy) setOpen(false);
                }}
              />
            </div>
          </div>,
          document.body,
        )}
    </>
  );
}
