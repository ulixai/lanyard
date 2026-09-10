import { useState } from "react";
import { LockKeyhole, ArrowUpRight } from "lucide-react";
import { api } from "../api";
import type { Snapshot } from "../types";
export function Unlock({
  snapshot,
  onReady,
  pending,
}: {
  snapshot: Snapshot;
  onReady: () => Promise<void>;
  pending: number;
}) {
  const [pin, setPin] = useState(""),
    [confirmation, setConfirmation] = useState(""),
    [oldPin, setOldPin] = useState(""),
    [fresh, setFresh] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const importing =
    !snapshot.initialized && snapshot.legacy_available && !fresh;
  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (busy) return;
    setError("");
    if (!snapshot.initialized && pin !== confirmation) {
      setError("The new PINs do not match.");
      return;
    }
    setBusy(true);
    try {
      await api(
        importing
          ? "import_legacy"
          : snapshot.initialized
            ? "unlock"
            : "initialize",
        importing ? { oldPin, newPin: pin } : { pin },
      );
      setPin("");
      setConfirmation("");
      setOldPin("");
      await onReady();
    } catch (e) {
      setError(String(e));
      setPin("");
      setConfirmation("");
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="unlock-layout">
      <section className="unlock-editorial">
        <span className="eyebrow">
          <LockKeyhole size={14} />
          ULIX / LANYARD
        </span>
        <h1>
          Your credentials.
          <br />
          <em>Your control.</em>
        </h1>
        <p>A private vault for the tools you trust.</p>
        <span className="system-readout">
          ENCRYPTED LOCALLY / SHARED WITH CONSENT
        </span>
      </section>
      <form className="unlock-form" onSubmit={(e) => void submit(e)}>
        <p className="eyebrow">
          {snapshot.initialized
            ? "VAULT LOCKED"
            : importing
              ? "YOUR EXISTING VAULT"
              : "WELCOME TO LANYARD"}
        </p>
        <h2>
          {snapshot.initialized
            ? "Welcome back."
            : importing
              ? "Bring your vault."
              : "Create your vault."}
        </h2>
        {importing && (
          <>
            <p>
              Your original vault stays untouched. Existing apps will ask for
              access again.
            </p>
            <label>
              Current Lanyard PIN
              <input
                type="password"
                autoComplete="current-password"
                value={oldPin}
                onChange={(e) => setOldPin(e.target.value)}
                maxLength={256}
              />
            </label>
          </>
        )}
        <label>
          {snapshot.initialized ? "PIN or passphrase" : "New PIN or passphrase"}
          <input
            type="password"
            autoFocus
            autoComplete={
              snapshot.initialized ? "current-password" : "new-password"
            }
            value={pin}
            onChange={(e) => setPin(e.target.value)}
            required
            minLength={snapshot.initialized ? undefined : 4}
            maxLength={256}
          />
        </label>
        {!snapshot.initialized && (
          <>
            <label>
              Confirm new PIN
              <input
                type="password"
                autoComplete="new-password"
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
                required
                maxLength={256}
              />
            </label>
            <p className="muted">
              Choose a strong passphrase or a long PIN. Lanyard cannot recover a
              forgotten PIN.
            </p>
          </>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <button className="primary block" disabled={busy}>
          {busy
            ? "Opening vault…"
            : snapshot.initialized
              ? "Unlock vault"
              : importing
                ? "Import and unlock"
                : "Create vault"}
          <ArrowUpRight size={17} />
        </button>
        {snapshot.legacy_available && !snapshot.initialized && (
          <button
            type="button"
            className="text-button"
            disabled={busy}
            onClick={() => setFresh((v) => !v)}
          >
            {fresh
              ? "Import my existing vault"
              : "Start a separate, empty vault"}
          </button>
        )}
        {pending > 0 && (
          <p className="access-waiting">
            {pending} application requests waiting. Unlock to review access.
          </p>
        )}
      </form>
    </div>
  );
}
