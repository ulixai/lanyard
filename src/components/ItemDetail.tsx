import { useEffect, useState } from "react";
import { Copy, Eye, EyeOff, Pencil, Trash2 } from "lucide-react";
import { api } from "../api";
import { categoryName, type Fields, type Item } from "../types";
import { Modal } from "./Modal";
export function ItemDetail({
  item,
  onClose,
  onEdit,
  onDelete,
}: {
  item: Item;
  onClose: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const [fields, setFields] = useState<Fields>({}),
    [visible, setVisible] = useState<Set<string>>(new Set()),
    [error, setError] = useState(""),
    [copied, setCopied] = useState("");
  useEffect(() => {
    let active = true;
    void api<Fields>("reveal", { id: item.id })
      .then((data) => {
        if (active) setFields(data);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [item.id]);
  const copy = async (key: string) => {
    try {
      await api("copy_value", { value: fields[key] });
      setCopied(key);
      setTimeout(() => setCopied(""), 2000);
    } catch (e) {
      setError(String(e));
    }
  };
  return (
    <Modal title={item.title} onClose={onClose} wide>
      <div className="modal-body">
        <p className="eyebrow">
          {categoryName(item.category)} / {item.fields.length} FIELDS
        </p>
        <div className="secret-list">
          {Object.entries(fields).map(([key, value]) => (
            <div className="secret-row" key={key}>
              <div className="secret-heading">
                <span>{key}</span>
                <div>
                  <button
                    className="icon-button"
                    aria-label={`${visible.has(key) ? "Hide" : "Reveal"} ${key}`}
                    onClick={() =>
                      setVisible((v) => {
                        const next = new Set(v);
                        if (next.has(key)) next.delete(key);
                        else next.add(key);
                        return next;
                      })
                    }
                  >
                    {visible.has(key) ? (
                      <EyeOff size={15} />
                    ) : (
                      <Eye size={15} />
                    )}
                  </button>
                  <button
                    className="icon-button"
                    aria-label={`Copy ${key}`}
                    onClick={() => void copy(key)}
                  >
                    <Copy size={15} />
                  </button>
                </div>
              </div>
              <pre className={visible.has(key) ? "" : "masked"}>
                {visible.has(key) ? value || "(empty)" : "••••••••••••••••"}
              </pre>
              {copied === key && (
                <span className="eyebrow" role="status">
                  COPIED / CLEARS IN 30 SECONDS
                </span>
              )}
            </div>
          ))}
        </div>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
      </div>
      <footer className="modal-actions">
        <button className="danger" onClick={onDelete}>
          <Trash2 size={15} />
          Delete
        </button>
        <button className="primary" onClick={onEdit}>
          <Pencil size={15} />
          Edit credential
        </button>
      </footer>
    </Modal>
  );
}
