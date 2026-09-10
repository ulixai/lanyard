import { useState } from "react";
import { UpdateButton } from "@ulix/update-react";
import { api } from "../api";
import type { Snapshot } from "../types";
import { Modal } from "./Modal";
export function Settings({
  snapshot,
  onClose,
  onSaved,
  onThemes,
}: {
  snapshot: Snapshot;
  onClose: () => void;
  onSaved: () => Promise<void>;
  onThemes: () => void;
}) {
  const [current, setCurrent] = useState(""),
    [next, setNext] = useState(""),
    [confirm, setConfirm] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState(""),
    [legacyPin, setLegacyPin] = useState(""),
    [legacyError, setLegacyError] = useState(""),
    [legacyMessage, setLegacyMessage] = useState(""),
    [skipMissing, setSkipMissing] = useState(false),
    [missingItems, setMissingItems] = useState<{ id: string; title: string }[]>([]);
  const importLegacy = async (e: React.FormEvent) => {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setLegacyError("");
    setLegacyMessage("");
    setMissingItems([]);
    try {
      const report = await api<{ imported: number; skipped: number; projects_added: number; missing: { id: string; title: string }[] }>(
        "import_legacy", { oldPin: legacyPin, newPin: "", skipMissing },
      );
      setMissingItems(report.missing);
      setLegacyMessage(
        `Imported ${report.imported} credentials and ${report.projects_added} projects. ` +
        `${report.skipped} credentials with existing IDs were kept unchanged.`,
      );
      await onSaved();
    } catch (e) {
      setLegacyError(String(e));
    } finally {
      setLegacyPin("");
      setBusy(false);
    }
  };
  const change = async (e: React.FormEvent) => {
    e.preventDefault();
    if (next !== confirm) {
      setError("The new PINs do not match.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      await api("change_pin", { current, next });
      setCurrent("");
      setNext("");
      setConfirm("");
      setMessage("PIN updated. Your credentials remain encrypted.");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const action = async (name: string, args: Record<string, unknown>) => {
    setBusy(true);
    setError("");
    try {
      await api(name, args);
      await onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal title="Settings & access" onClose={onClose} busy={busy} wide>
      <div className="modal-body">
        <section className="settings-section">
          <span className="eyebrow">01 / YOUR DESKTOP</span>
          <div className="settings-row">
            <div>
              <h3>Appearance</h3>
              <p className="muted">Presets and custom palettes.</p>
            </div>
            <button disabled={busy} onClick={onThemes}>Choose theme</button>
          </div>
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={snapshot.close_to_tray}
              disabled={busy}
              onChange={(e) =>
                void action("set_close_to_tray", { value: e.target.checked })
              }
            />
            Keep Lanyard in the tray when the window closes
          </label>
        </section>
        <section className="settings-section">
          <span className="eyebrow">02 / VAULT SECURITY</span>
          <form onSubmit={(e) => void change(e)}>
            <div className="form-grid">
              <label className="span-two">
                Current PIN
                <input
                  type="password"
                  autoComplete="current-password"
                  required
                  maxLength={256}
                  value={current}
                  onChange={(e) => setCurrent(e.target.value)}
                />
              </label>
              <label>
                New PIN or passphrase
                <input
                  type="password"
                  autoComplete="new-password"
                  required
                  minLength={4}
                  maxLength={256}
                  value={next}
                  onChange={(e) => setNext(e.target.value)}
                />
              </label>
              <label>
                Confirm PIN
                <input
                  type="password"
                  autoComplete="new-password"
                  required
                  maxLength={256}
                  value={confirm}
                  onChange={(e) => setConfirm(e.target.value)}
                />
              </label>
            </div>
            <button disabled={busy}>Change PIN</button>
          </form>
        </section>
        <section className="settings-section">
          <span className="eyebrow">03 / CONNECTED APPLICATIONS</span>
          {snapshot.clients.length === 0 ? (
            <p className="muted">No applications have been paired.</p>
          ) : (
            snapshot.clients.map((client) => (
              <div className="client-row" key={client.id}>
                <div className="settings-row">
                  <div>
                    <h3>{client.name}</h3>
                    <code className="muted">{client.id}</code>
                  </div>
                  <button
                    disabled={busy}
                    onClick={() =>
                      void action("revoke", {
                        clientId: client.id,
                        itemId: null,
                      })
                    }
                  >
                    Forget app
                  </button>
                </div>
                {snapshot.grants
                  .filter((g) => g.client_id === client.id)
                  .map((g) => (
                    <div className="grant-row" key={g.item_id}>
                      <span>
                        {snapshot.items.find((i) => i.id === g.item_id)
                          ?.title ?? "Removed credential"}
                      </span>
                      <button
                        className="text-button"
                        disabled={busy}
                        onClick={() =>
                          void action("revoke", {
                            clientId: client.id,
                            itemId: g.item_id,
                          })
                        }
                      >
                        Revoke automatic access
                      </button>
                    </div>
                  ))}
              </div>
            ))
          )}
        </section>
        <section className="settings-section">
          <span className="eyebrow">04 / IMPORT LEGACY VAULT</span>
          <p className="muted">
            Add credentials from the original Python Lanyard app. Your current PIN,
            credentials and app permissions are retained. Items with existing IDs
            are skipped. Missing credentials stop the import unless you enable
            the option below. Other read or decryption errors always stop it.
          </p>
          {snapshot.legacy_available ? (
            <form onSubmit={(e) => void importLegacy(e)}>
              <label>
                PIN for the old Python vault
                <input
                  type="password"
                  autoComplete="off"
                  maxLength={256}
                  disabled={busy}
                  value={legacyPin}
                  onChange={(e) => setLegacyPin(e.target.value)}
                />
              </label>
              <p className="muted">Leave blank if your old vault had no PIN.</p>
              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={skipMissing}
                  disabled={busy}
                  onChange={(e) => setSkipMissing(e.target.checked)}
                />
                Import available credentials and list missing items
              </label>
              <p className="muted">
                Missing items will be left out of this import. Original entries
                stay untouched, and you can retry them later if their secrets
                become available.
              </p>
              <button className="primary" disabled={busy}>
                {busy ? "Please wait…" : "Import legacy credentials"}
              </button>
            </form>
          ) : (
            <p className="muted">
              No legacy vault was found in your user account's usual Lanyard data
              folder. If you restore its files there, restart Lanyard to detect them.
            </p>
          )}
          {legacyError && <p className="error" role="alert">{legacyError}</p>}
          {legacyMessage && <p role="status">{legacyMessage}</p>}
          {missingItems.length > 0 && (
            <div role="status">
              <p>{missingItems.length} missing credentials were not imported:</p>
              <ul>
                {missingItems.map((item) => (
                  <li key={item.id}>{item.title} — <code>{item.id}</code></li>
                ))}
              </ul>
              <p className="muted">Your original vault entries have not been deleted.</p>
            </div>
          )}
        </section>
        <section className="settings-section">
          <span className="eyebrow">05 / SOFTWARE</span>
          <div className="settings-row">
            <p>
              Lanyard 0.2.0 <span className="muted">/ ULIX AI</span>
            </p>
            <UpdateButton productName="Lanyard" />
          </div>
        </section>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {message && <p role="status">{message}</p>}
      </div>
      <footer className="modal-actions">
        <button className="primary" disabled={busy} onClick={onClose}>
          Done
        </button>
      </footer>
    </Modal>
  );
}
