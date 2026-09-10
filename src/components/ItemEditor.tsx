import { useEffect, useState } from "react";
import { Plus, Trash2, Upload, KeyRound } from "lucide-react";
import { api } from "../api";
import {
  categories,
  type Category,
  type Fields,
  type Item,
  type Project,
} from "../types";
import { Modal } from "./Modal";
type Field = { key: string; value: string; id: string };
const rows = (f: Fields): Field[] =>
  Object.entries(f).map(([key, value]) => ({
    key,
    value,
    id: crypto.randomUUID(),
  }));
export function ItemEditor({
  item,
  projectId,
  projects,
  onClose,
  onSaved,
}: {
  item?: Item;
  projectId: string | null;
  projects: Project[];
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const [title, setTitle] = useState(item?.title ?? ""),
    [category, setCategory] = useState<Category>(item?.category ?? "api_key"),
    [project, setProject] = useState(item?.project_id ?? projectId ?? ""),
    [fields, setFields] = useState<Field[]>(rows({ API_KEY: "" })),
    [busy, setBusy] = useState(!!item),
    [loaded, setLoaded] = useState(!item),
    [error, setError] = useState(""),
    [algorithm, setAlgorithm] = useState("ed25519");
  useEffect(() => {
    let active = true;
    if (item)
      void api<Fields>("reveal", { id: item.id })
        .then((data) => {
          if (active) {
            setFields(rows(data));
            setLoaded(true);
          }
        })
        .catch((e) => {
          if (active) setError(String(e));
        })
        .finally(() => {
          if (active) setBusy(false);
        });
    return () => {
      active = false;
    };
  }, [item]);
  const changeCategory = (next: Category) => {
    setCategory(next);
    if (fields.every((f) => !f.value))
      setFields(
        rows(
          Object.fromEntries(
            (categories.find((c) => c.id === next)?.fields ?? []).map((k) => [
              k,
              "",
            ]),
          ),
        ),
      );
  };
  const save = async (e: React.FormEvent) => {
    e.preventDefault();
    if (busy || !loaded) return;
    setError("");
    const names = fields.map((f) => f.key.trim());
    if (new Set(names).size !== names.length || names.some((n) => !n)) {
      setError("Each field needs a unique, nonempty name.");
      return;
    }
    setBusy(true);
    try {
      await api("save_item", {
        input: {
          id: item?.id ?? null,
          title,
          category,
          project_id: project || null,
          fields: Object.fromEntries(
            fields.map((f) => [f.key.trim(), f.value]),
          ),
        },
      });
      await onSaved();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const importEnv = async (file: File | undefined) => {
    if (!file) return;
    setBusy(true);
    setError("");
    try {
      if (file.size > 1024 * 1024)
        throw Error("Choose an environment file smaller than 1 MB.");
      const data = await api<Fields>("parse_env", { text: await file.text() });
      setFields(
        rows({
          ...Object.fromEntries(
            fields.filter((f) => f.value).map((f) => [f.key, f.value]),
          ),
          ...data,
        }),
      );
      if (!title) setTitle(file.name.replace(/^\./, ""));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const generate = async () => {
    setBusy(true);
    setError("");
    try {
      setFields(rows(await api<Fields>("generate_keypair", { algorithm })));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title={item ? "Edit credential" : "Add credential"}
      onClose={onClose}
      wide
      busy={busy}
    >
      <form onSubmit={(e) => void save(e)}>
        <div className="modal-body">
          <div className="form-grid">
            <label className="span-two">
              Title
              <input
                required
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                maxLength={256}
                placeholder="e.g. Production API"
              />
            </label>
            <label>
              Category
              <select
                value={category}
                onChange={(e) => changeCategory(e.target.value as Category)}
                disabled={busy}
              >
                {categories.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.singular}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Location
              <select
                value={project}
                onChange={(e) => setProject(e.target.value)}
              >
                <option value="">Base vault</option>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.title}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {category === "env" && (
            <div className="editor-tools">
              <label className="file-button">
                <Upload size={15} />
                Import .env file
                <input
                  type="file"
                  accept=".env,.txt,*/*"
                  disabled={busy}
                  onChange={(e) => {
                    void importEnv(e.target.files?.[0]);
                    e.target.value = "";
                  }}
                />
              </label>
              <span className="muted">
                Imported names replace matching fields.
              </span>
            </div>
          )}
          {category === "crypto" && (
            <div className="editor-tools">
              <select
                aria-label="Key algorithm"
                value={algorithm}
                onChange={(e) => setAlgorithm(e.target.value)}
                disabled={busy}
              >
                <option value="ed25519">Ed25519</option>
                <option value="rsa-2048">RSA 2048</option>
                <option value="rsa-4096">RSA 4096</option>
              </select>
              <button
                type="button"
                disabled={busy}
                onClick={() => void generate()}
              >
                <KeyRound size={15} />
                {busy ? "Generating…" : "Generate key pair"}
              </button>
              <span className="muted">
                Replaces the fields below. You can also paste keys manually.
              </span>
            </div>
          )}
          <div className="section-heading">
            <span className="eyebrow">SECRET FIELDS / {fields.length}</span>
            <button
              type="button"
              disabled={busy || fields.length >= 256}
              onClick={() =>
                setFields((v) => [
                  ...v,
                  { id: crypto.randomUUID(), key: "", value: "" },
                ])
              }
            >
              <Plus size={14} />
              Add field
            </button>
          </div>
          <div className="field-list">
            {fields.map((f, index) => (
              <div key={f.id} className="field-editor">
                <div>
                  <input
                    aria-label={`Field ${index + 1} name`}
                    value={f.key}
                    maxLength={256}
                    required
                    placeholder="Field name"
                    onChange={(e) =>
                      setFields((v) =>
                        v.map((row) =>
                          row.id === f.id
                            ? { ...row, key: e.target.value }
                            : row,
                        ),
                      )
                    }
                  />
                  <button
                    type="button"
                    aria-label={`Remove ${f.key || "field"}`}
                    className="icon-button"
                    disabled={busy || fields.length === 1}
                    onClick={() =>
                      setFields((v) => v.filter((row) => row.id !== f.id))
                    }
                  >
                    <Trash2 size={15} />
                  </button>
                </div>
                <textarea
                  aria-label={`${f.key || `Field ${index + 1}`} value`}
                  value={f.value}
                  placeholder="Value"
                  spellCheck={false}
                  autoComplete="off"
                  rows={
                    category === "crypto" || category === "recovery" ? 5 : 2
                  }
                  onChange={(e) =>
                    setFields((v) =>
                      v.map((row) =>
                        row.id === f.id
                          ? { ...row, value: e.target.value }
                          : row,
                      ),
                    )
                  }
                />
              </div>
            ))}
          </div>
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
          <button className="primary" disabled={busy || !loaded}>
            {busy ? "Working…" : "Save credential"}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
