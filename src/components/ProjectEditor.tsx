import { useState } from "react";
import { api } from "../api";
import type { Project } from "../types";
import { Modal } from "./Modal";
export function ProjectEditor({
  project,
  onClose,
  onSaved,
}: {
  project?: Project;
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const [title, setTitle] = useState(project?.title ?? ""),
    [description, setDescription] = useState(project?.description ?? ""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const save = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await api("save_project", {
        id: project?.id ?? null,
        title,
        description,
      });
      await onSaved();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title={project ? "Edit project" : "Create project"}
      onClose={onClose}
      busy={busy}
    >
      <form onSubmit={(e) => void save(e)}>
        <div className="modal-body">
          <label>
            Project name
            <input
              required
              value={title}
              maxLength={256}
              onChange={(e) => setTitle(e.target.value)}
            />
          </label>
          <label>
            Description
            <textarea
              value={description}
              maxLength={8192}
              rows={4}
              onChange={(e) => setDescription(e.target.value)}
            />
          </label>
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
        </div>
        <footer className="modal-actions">
          <button type="button" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button className="primary" disabled={busy}>
            Save project
          </button>
        </footer>
      </form>
    </Modal>
  );
}
