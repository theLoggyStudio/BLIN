import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { formatDateTimeFr } from "@/lib/formatDateTime";

interface IoLogSummary {
  userName: string;
  importCount: number;
  exportCount: number;
  importObjects: number;
  exportObjects: number;
}

interface IoLogEntry {
  kind: string;
  entityKey: string;
  entityLabel: string;
  userName: string;
  objectCount: number;
  createdAt: string;
}

interface ScreenGroup {
  key: string;
  label: string;
  imports: number;
  exports: number;
  importObjects: number;
  exportObjects: number;
  events: IoLogEntry[];
}

/** Journal des imports / exports CSV : tableau par importateur + détail par écran. */
export function ImportExportPanel() {
  const [rows, setRows] = useState<IoLogSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [detail, setDetail] = useState<IoLogEntry[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<IoLogSummary[]>("io_log_summary");
      setRows(data);
    } catch (e) {
      setError(String(e));
      setRows([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const openDetail = useCallback(async (userName: string) => {
    setSelected(userName);
    setDetailLoading(true);
    setDetail([]);
    try {
      const data = await invoke<IoLogEntry[]>("io_log_detail", {
        payload: { user_name: userName },
      });
      setDetail(data);
    } catch {
      setDetail([]);
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const grouped = useMemo<ScreenGroup[]>(() => {
    const map = new Map<string, ScreenGroup>();
    for (const e of detail) {
      const g =
        map.get(e.entityKey) ?? {
          key: e.entityKey,
          label: e.entityLabel || e.entityKey,
          imports: 0,
          exports: 0,
          importObjects: 0,
          exportObjects: 0,
          events: [],
        };
      if (e.kind === "import") {
        g.imports += 1;
        g.importObjects += e.objectCount;
      } else {
        g.exports += 1;
        g.exportObjects += e.objectCount;
      }
      g.events.push(e);
      map.set(e.entityKey, g);
    }
    return Array.from(map.values());
  }, [detail]);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted">
          Cliquez sur une ligne pour voir le détail par écran (imports / exports, objets, dates).
        </p>
        <Button variant="ghost" size="sm" onClick={() => void load()} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {error && <Alert variant="danger" size="box" message={error} />}

      {loading && rows.length === 0 ? (
        <p className="text-sm text-muted">Chargement…</p>
      ) : rows.length === 0 ? (
        <Alert
          variant="info"
          size="box"
          message="Aucun import ou export enregistré pour le moment."
        />
      ) : (
        <div className="overflow-hidden rounded-lg border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-surface-elevated text-left text-muted">
                <th className="px-3 py-2 font-medium">Nom de l&apos;importateur</th>
                <th className="px-3 py-2 text-right font-medium">Nombre d&apos;importations</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr
                  key={r.userName}
                  className="cursor-pointer border-b border-border/40 last:border-0 hover:bg-surface-elevated"
                  onClick={() => void openDetail(r.userName)}
                >
                  <td className="px-3 py-2 text-foreground">{r.userName || "—"}</td>
                  <td className="px-3 py-2 text-right text-foreground">{r.importCount}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <Modal
        open={selected !== null}
        onClose={() => setSelected(null)}
        title={`Imports / exports — ${selected ?? ""}`}
        size="lg"
      >
        <div className="space-y-4">
          <p className="text-sm text-muted">
            Importateur : <span className="text-foreground">{selected ?? "—"}</span>
          </p>

          {detailLoading ? (
            <p className="text-sm text-muted">Chargement du détail…</p>
          ) : grouped.length === 0 ? (
            <p className="text-sm text-muted">Aucun mouvement pour cet utilisateur.</p>
          ) : (
            grouped.map((g) => (
              <div key={g.key} className="rounded-lg border border-border p-3">
                <p className="font-medium text-foreground">{g.label}</p>
                <p className="mt-1 text-sm text-muted">
                  Imports : {g.imports} ({g.importObjects} objet(s) importé(s)) — Exports :{" "}
                  {g.exports} ({g.exportObjects} objet(s) exporté(s))
                </p>
                <ul className="mt-2 space-y-1 text-sm text-foreground">
                  {g.events.map((e, i) => (
                    <li key={`${g.key}-${i}`}>
                      {e.kind === "import" ? "Import" : "Export"} de {e.objectCount} objet(s) le{" "}
                      {formatDateTimeFr(e.createdAt)} par {e.userName || "—"}
                    </li>
                  ))}
                </ul>
              </div>
            ))
          )}
        </div>
      </Modal>
    </div>
  );
}
