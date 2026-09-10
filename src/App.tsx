import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ArrowUpRight,
  ChevronRight,
  Folder,
  KeyRound,
  LockKeyhole,
  Minus,
  Plus,
  Search,
  Settings2,
  Square,
  X,
  Pencil,
  Trash2,
} from "lucide-react";
import { api } from "./api";
import {
  categories,
  categoryName,
  type AccessRequest,
  type Category,
  type Item,
  type Project,
  type Snapshot,
} from "./types";
import { applyTheme, initialTheme } from "./themes";
import { Unlock } from "./components/Unlock";
import { Confirm } from "./components/Modal";
import { ItemEditor } from "./components/ItemEditor";
import { ItemDetail } from "./components/ItemDetail";
import { ProjectEditor } from "./components/ProjectEditor";
import { Settings } from "./components/Settings";
import { ThemeEditor } from "./components/ThemeEditor";
import { AccessPrompt } from "./components/AccessPrompt";
type Dialog =
  | { kind: "item"; item?: Item }
  | { kind: "detail"; item: Item }
  | { kind: "project"; project?: Project }
  | { kind: "delete-item"; item: Item }
  | { kind: "delete-project"; project: Project }
  | { kind: "settings" }
  | { kind: "themes" }
  | null;
export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null),
    [requests, setRequests] = useState<AccessRequest[]>([]),
    [error, setError] = useState(""),
    [dialog, setDialog] = useState<Dialog>(null),
    [projectId, setProjectId] = useState<string | null>(null),
    [category, setCategory] = useState<Category | null>(null),
    [search, setSearch] = useState(""),
    [theme, setTheme] = useState(initialTheme);
  const refreshId = useRef(0);
  const refresh = useCallback(async () => {
    const id = ++refreshId.current;
    const [next, pending] = await Promise.all([
      api<Snapshot>("snapshot"),
      api<AccessRequest[]>("requests"),
    ]);
    if (id !== refreshId.current) return;
    setSnapshot(next);
    setRequests(pending);
    if (next.locked) {
      setDialog(null);
      setSearch("");
    }
    setError("");
  }, []);
  useEffect(() => {
    applyTheme(theme);
  }, [theme]);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void refresh().catch((e) => setError(String(e)));
    void listen("lanyard-changed", () => {
      void refresh().catch((e) => {
        if (active) setError(String(e));
      });
    })
      .then((fn) => {
        if (active) stop = fn;
        else fn();
      })
      .catch((e) => setError(String(e)));
    const tick = setInterval(() => {
      void api<AccessRequest[]>("requests")
        .then((v) => {
          if (active) setRequests(v);
        })
        .catch(() => {});
    }, 1000);
    return () => {
      active = false;
      stop?.();
      clearInterval(tick);
    };
  }, [refresh]);
  const action = async (name: string, args: Record<string, unknown> = {}) => {
    try {
      await api(name, args);
      await refresh();
      setDialog(null);
    } catch (e) {
      setError(String(e));
      throw e;
    }
  };
  const windowAction = async (
    name: "minimize" | "toggleMaximize" | "close",
  ) => {
    try {
      await getCurrentWindow()[name]();
    } catch (e) {
      setError(String(e));
    }
  };
  const project = snapshot?.projects.find((p) => p.id === projectId);
  const items = (snapshot?.items ?? []).filter(
    (i) =>
      i.project_id === projectId &&
      (!category || i.category === category) &&
      `${i.title} ${i.fields.join(" ")}`
        .toLowerCase()
        .includes(search.toLowerCase()),
  );
  return (
    <div className="app-shell">
      <header className="titlebar">
        <span className="brand" data-tauri-drag-region>
          LANYARD<span>//</span>VAULT
        </span>
        <div className="titlebar-drag" data-tauri-drag-region />
        <div className="window-controls">
          <button
            aria-label="Minimize window"
            onClick={() => void windowAction("minimize")}
          >
            <Minus size={14} />
          </button>
          <button
            aria-label="Maximize or restore window"
            onClick={() => void windowAction("toggleMaximize")}
          >
            <Square size={12} />
          </button>
          <button
            aria-label="Close window"
            onClick={() => void windowAction("close")}
          >
            <X size={15} />
          </button>
        </div>
      </header>
      {error && (
        <div className="global-error" role="alert">
          {error}
          <button
            onClick={() => void refresh().catch((e) => setError(String(e)))}
          >
            Retry
          </button>
        </div>
      )}
      {!snapshot ? (
        <div className="loading">
          <span className="eyebrow">OPENING LANYARD</span>
        </div>
      ) : snapshot.locked ? (
        <Unlock
          snapshot={snapshot}
          onReady={refresh}
          pending={requests.length}
        />
      ) : (
        <div className="workspace">
          <aside className="sidebar">
            <div className="sidebar-top">
              <span className="eyebrow">YOUR VAULT</span>
              <button
                className="navigation"
                aria-current={projectId === null ? "page" : undefined}
                onClick={() => {
                  setProjectId(null);
                  setCategory(null);
                }}
              >
                <KeyRound size={16} />
                Base vault
                <span className="nav-count">
                  {snapshot.items.filter((i) => !i.project_id).length}
                </span>
              </button>
            </div>
            <div className="sidebar-projects">
              <div className="section-heading">
                <span className="eyebrow">PROJECTS</span>
                <button
                  className="icon-button"
                  aria-label="Create project"
                  onClick={() => setDialog({ kind: "project" })}
                >
                  <Plus size={15} />
                </button>
              </div>
              {snapshot.projects.map((p) => (
                <button
                  className="navigation"
                  key={p.id}
                  aria-current={projectId === p.id ? "page" : undefined}
                  onClick={() => {
                    setProjectId(p.id);
                    setCategory(null);
                  }}
                >
                  <Folder size={15} />
                  <span className="truncate">{p.title}</span>
                  <span className="nav-count">
                    {snapshot.items.filter((i) => i.project_id === p.id).length}
                  </span>
                </button>
              ))}
              {snapshot.projects.length === 0 && (
                <p className="sidebar-note">Group credentials by project.</p>
              )}
            </div>
            <footer className="sidebar-bottom">
              <button
                className="navigation"
                onClick={() => setDialog({ kind: "settings" })}
              >
                <Settings2 size={16} />
                Settings & access
              </button>
              <button
                className="navigation"
                onClick={() => void action("lock").catch(() => {})}
              >
                <LockKeyhole size={16} />
                Lock vault
              </button>
              <span className="system-readout">
                <i />
                LOCAL / CONNECTED
              </span>
            </footer>
          </aside>
          <main className="vault-main">
            <div className="page-heading">
              <div>
                <span className="eyebrow">
                  SECURED BY YOUR SYSTEM / {project ? "PROJECT" : "BASE VAULT"}
                </span>
                <h1>{project?.title ?? "Your vault."}</h1>
                <p className="muted">
                  {project?.description ||
                    "A place for the credentials that keep you moving."}
                </p>
              </div>
              <div className="page-actions">
                {project && (
                  <>
                    <button
                      className="icon-button"
                      aria-label="Edit project"
                      onClick={() => setDialog({ kind: "project", project })}
                    >
                      <Pencil size={16} />
                    </button>
                    <button
                      className="icon-button"
                      aria-label="Delete project"
                      onClick={() =>
                        setDialog({ kind: "delete-project", project })
                      }
                    >
                      <Trash2 size={16} />
                    </button>
                  </>
                )}
                <button
                  className="primary"
                  onClick={() => setDialog({ kind: "item" })}
                >
                  Add credential
                  <Plus size={16} />
                </button>
              </div>
            </div>
            <div className="vault-toolbar">
              <div className="category-tabs">
                <button
                  aria-pressed={!category}
                  onClick={() => setCategory(null)}
                >
                  All
                </button>
                {categories.map((c) => (
                  <button
                    key={c.id}
                    aria-pressed={category === c.id}
                    onClick={() => setCategory(c.id)}
                  >
                    {c.name}
                  </button>
                ))}
              </div>
              <label className="search">
                <Search size={15} />
                <input
                  aria-label="Search credentials"
                  placeholder="Search credentials"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
              </label>
            </div>
            <div className="inventory-heading">
              <span className="eyebrow">
                CREDENTIAL / {items.length.toString().padStart(2, "0")}
              </span>
              <span className="eyebrow">CATEGORY</span>
            </div>
            <div className="inventory">
              {items.map((item) => (
                <button
                  className="inventory-row"
                  key={item.id}
                  onClick={() => setDialog({ kind: "detail", item })}
                >
                  <span className="item-symbol">
                    <KeyRound size={19} />
                  </span>
                  <span className="item-name">
                    <strong>{item.title}</strong>
                    <small>
                      {item.fields.length}{" "}
                      {item.fields.length === 1 ? "field" : "fields"}
                    </small>
                  </span>
                  <span className="item-category">
                    {categoryName(item.category)}
                  </span>
                  <ChevronRight size={16} />
                </button>
              ))}
            </div>
            {items.length === 0 && (
              <div className="empty-state">
                {/* <span className="eyebrow">SPACE FOR WHAT’S NEXT</span> */}
                <h2>
                  {search ? "Nothing matches." : "Start with a credential."}
                </h2>
                <p>
                  {search
                    ? "Try another name or field."
                    : "Add an API key, password, license, environment, recovery code, or key pair."}
                </p>
                {!search && (
                  <button
                    className="text-button"
                    onClick={() => setDialog({ kind: "item" })}
                  >
                    Add your first credential <ArrowUpRight size={16} />
                  </button>
                )}
              </div>
            )}
            <footer className="vault-footer">
              <span className="eyebrow">ENCRYPTED LOCALLY</span>
              <span className="eyebrow">ULIX AI / 0.2.0</span>
            </footer>
          </main>
        </div>
      )}
      {snapshot && !snapshot.locked && (
        <>
          {dialog?.kind === "item" && (
            <ItemEditor
              item={dialog.item}
              projectId={projectId}
              projects={snapshot.projects}
              onClose={() => setDialog(null)}
              onSaved={refresh}
            />
          )}{" "}
          {dialog?.kind === "detail" && (
            <ItemDetail
              item={dialog.item}
              onClose={() => setDialog(null)}
              onEdit={() => setDialog({ kind: "item", item: dialog.item })}
              onDelete={() =>
                setDialog({ kind: "delete-item", item: dialog.item })
              }
            />
          )}{" "}
          {dialog?.kind === "project" && (
            <ProjectEditor
              project={dialog.project}
              onClose={() => setDialog(null)}
              onSaved={refresh}
            />
          )}{" "}
          {dialog?.kind === "delete-item" && (
            <Confirm
              title={`Delete ${dialog.item.title}?`}
              detail="This permanently removes the credential and its saved app permissions."
              onClose={() => setDialog(null)}
              onConfirm={() => action("delete_item", { id: dialog.item.id })}
            />
          )}{" "}
          {dialog?.kind === "delete-project" && (
            <Confirm
              title={`Delete ${dialog.project.title}?`}
              detail={`This permanently deletes the project and all ${snapshot.items.filter((i) => i.project_id === dialog.project.id).length} credentials inside it.`}
              onClose={() => setDialog(null)}
              onConfirm={async () => {
                await action("delete_project", { id: dialog.project.id });
                setProjectId(null);
              }}
            />
          )}{" "}
          {dialog?.kind === "settings" && (
            <Settings
              snapshot={snapshot}
              onClose={() => setDialog(null)}
              onSaved={refresh}
              onThemes={() => setDialog({ kind: "themes" })}
            />
          )}{" "}
          {dialog?.kind === "themes" && (
            <ThemeEditor
              theme={theme}
              onChange={setTheme}
              onClose={() => setDialog(null)}
            />
          )}{" "}
          {!dialog && requests[0] && (
            <AccessPrompt
              key={requests[0].id}
              request={requests[0]}
              snapshot={snapshot}
              onDone={refresh}
            />
          )}
        </>
      )}
    </div>
  );
}
