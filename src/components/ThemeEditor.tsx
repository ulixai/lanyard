import { useState } from "react";
import {
  applyTheme,
  presets,
  readThemes,
  type Palette,
  type Theme,
} from "../themes";
import { Modal } from "./Modal";
export function ThemeEditor({
  theme,
  onChange,
  onClose,
}: {
  theme: Theme;
  onChange: (theme: Theme) => void;
  onClose: () => void;
}) {
  const [custom, setCustom] = useState(readThemes),
    [editing, setEditing] = useState<Theme | null>(null),
    [error, setError] = useState("");
  const persist = (next: Theme[]) => {
    localStorage.setItem("lanyard.custom-themes.v2", JSON.stringify(next));
    setCustom(next);
  };
  const select = (value: Theme) => {
    applyTheme(value);
    onChange(value);
  };
  const save = () => {
    if (!editing) return;
    try {
      if (!editing.name.trim()) throw Error("Give your theme a name.");
      if (
        !Object.values(editing.colors).every((v) => /^#[0-9a-fA-F]{6}$/.test(v))
      )
        throw Error("Use six-digit hex colors.");
      const next = { ...editing, name: editing.name.trim(), custom: true };
      persist([...custom.filter((t) => t.id !== next.id), next]);
      select(next);
      setEditing(null);
    } catch (e) {
      setError(String(e));
    }
  };
  return (
    <Modal title="Make it yours." onClose={onClose} wide>
      <div className="modal-body">
        <p className="muted">Your desktop, your palette.</p>
        <div className="theme-grid">
          {[...presets, ...custom].map((t) => (
            <button
              key={t.id}
              aria-pressed={theme.id === t.id}
              className="theme-option"
              onClick={() => select(t)}
            >
              <span
                className="theme-swatch"
                style={{
                  background: t.colors.background,
                  color: t.colors.foreground,
                  borderColor: t.colors.border,
                }}
              >
                <span style={{ background: t.colors.surface }}>Aa</span>
                <i style={{ background: t.colors.accent }} />
              </span>
              <span>{t.name}</span>
            </button>
          ))}
        </div>
        {theme.custom && (
          <div className="editor-tools">
            <button
              onClick={() => {
                setEditing({ ...theme, colors: { ...theme.colors } });
                setError("");
              }}
            >
              Edit selected theme
            </button>
            <button
              className="danger"
              onClick={() => {
                try {
                  persist(custom.filter((t) => t.id !== theme.id));
                  select(presets[0]);
                } catch (e) {
                  setError(String(e));
                }
              }}
            >
              Delete selected theme
            </button>
          </div>
        )}
        {editing && (
          <section className="theme-editor">
            <label>
              Theme name
              <input
                value={editing.name}
                maxLength={60}
                onChange={(e) =>
                  setEditing({ ...editing, name: e.target.value })
                }
              />
            </label>
            <div className="form-grid">
              {Object.entries(editing.colors).map(([key, value]) => (
                <label key={key}>
                  {key}
                  <div className="color-input">
                    <input
                      aria-label={`${key} color picker`}
                      type="color"
                      value={
                        /^#[0-9a-fA-F]{6}$/.test(value) ? value : "#000000"
                      }
                      onChange={(e) =>
                        setEditing({
                          ...editing,
                          colors: { ...editing.colors, [key]: e.target.value },
                        })
                      }
                    />
                    <input
                      aria-label={`${key} hex color`}
                      value={value}
                      maxLength={7}
                      onChange={(e) =>
                        setEditing({
                          ...editing,
                          colors: {
                            ...editing.colors,
                            [key as keyof Palette]: e.target.value,
                          },
                        })
                      }
                    />
                  </div>
                </label>
              ))}
            </div>
            <div className="editor-tools">
              <button className="primary" onClick={save}>
                Save theme
              </button>
              <button onClick={() => setEditing(null)}>Cancel editing</button>
            </div>
          </section>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
      </div>
      <footer className="modal-actions">
        <button
          disabled={custom.length >= 30}
          onClick={() => {
            setError("");
            setEditing({
              id: `custom-${crypto.randomUUID()}`,
              name: `${theme.name} custom`,
              colors: { ...theme.colors },
              custom: true,
            });
          }}
        >
          Create custom theme
        </button>
        <button className="primary" onClick={onClose}>
          Done
        </button>
      </footer>
    </Modal>
  );
}
