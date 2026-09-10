import { useState } from "react";
import { ShieldCheck } from "lucide-react";
import { api } from "../api";
import { categoryName, type AccessRequest, type Snapshot } from "../types";
import { Modal } from "./Modal";
export function AccessPrompt({
  request,
  snapshot,
  onDone,
}: {
  request: AccessRequest;
  snapshot: Snapshot;
  onDone: () => Promise<void>;
}) {
  const [selected, setSelected] = useState(request.target_id ?? ""),
    [always, setAlways] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const items = snapshot.items.filter(
    (i) =>
      (!request.target_id || i.id === request.target_id) &&
      (!request.category || i.category === request.category),
  );
  const respond = async (allow: boolean) => {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      await api("respond", {
        requestId: request.id,
        targetId: allow ? selected : null,
        always: allow && always,
      });
      await onDone();
    } catch (e) {
      setError(String(e));
      await onDone();
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title="An app is asking."
      onClose={() => void respond(false)}
      busy={busy}
    >
      <div className="modal-body">
        <span className="eyebrow">
          <ShieldCheck size={14} />
          CREDENTIAL ACCESS / {request.expires_in}s
        </span>
        <h3 className="request-app">{request.app_name}</h3>
        <p>
          {request.paired
            ? "Paired application"
            : "New application — approve only if you initiated this request."}
        </p>
        <code className="muted client-id">{request.client_id}</code>
        {request.reason && <blockquote>{request.reason}</blockquote>}
        <label>
          Credential
          <select
            value={selected}
            onChange={(e) => setSelected(e.target.value)}
            disabled={!!request.target_id || busy}
          >
            <option value="">Choose a credential…</option>
            {[{ id: "", title: "Base vault" }, ...snapshot.projects].map(
              (p) => {
                const group = items.filter(
                  (i) => (i.project_id ?? "") === p.id,
                );
                return (
                  group.length > 0 && (
                    <optgroup key={p.id} label={p.title}>
                      {group.map((i) => (
                        <option key={i.id} value={i.id}>
                          {i.title} · {categoryName(i.category)}
                        </option>
                      ))}
                    </optgroup>
                  )
                );
              },
            )}
          </select>
        </label>
        {items.length === 0 && (
          <p className="error">No matching credential is available.</p>
        )}
        <label className="checkbox-row">
          <input
            type="checkbox"
            checked={always}
            onChange={(e) => setAlways(e.target.checked)}
            disabled={busy}
          />
          Allow this app to access this credential without asking again
        </label>
        <p className="muted">
          Approval shares all fields in the selected credential. You can revoke
          access in Settings.
        </p>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
      </div>
      <footer className="modal-actions">
        <button disabled={busy} onClick={() => void respond(false)}>
          Deny
        </button>
        <button
          className="primary"
          disabled={busy || !items.some((i) => i.id === selected)}
          onClick={() => void respond(true)}
        >
          Allow {always ? "and remember" : "once"}
        </button>
      </footer>
    </Modal>
  );
}
