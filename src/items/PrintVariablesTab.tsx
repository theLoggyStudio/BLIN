import { useState } from "react";
import { Text } from "@/items/Text";
import {
  buildVariableCatalog,
  formatTableBlockToken,
  formatVariableToken,
  type EntityVariableCatalog,
} from "@/lib/print/templateVariables";
import type { EntityDef } from "@/types/entity";

interface PrintVariablesTabProps {
  entities: EntityDef[];
  primaryTableKey?: string | null;
}

/** Référence des variables {{table.champ}} disponibles. */
export function PrintVariablesTab({ entities, primaryTableKey }: PrintVariablesTabProps) {
  const catalog: EntityVariableCatalog = buildVariableCatalog(entities);
  const [copied, setCopied] = useState<string | null>(null);

  const copy = async (token: string) => {
    try {
      await navigator.clipboard.writeText(token);
      setCopied(token);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      setCopied(null);
    }
  };

  return (
    <div className="space-y-4 max-h-[480px] overflow-y-auto pr-1">
      <Text variant="muted" className="text-sm">
        Syntaxe : <code className="text-secondary">{`{{nomTable.nomVariable}}`}</code>. Dans HTML ou
        CSS, tapez <code className="text-secondary">{`{{`}</code> pour afficher les tables
        existantes, puis choisissez un champ après le point.
      </Text>
      {primaryTableKey && (
        <p className="text-xs text-amber-400/90">
          À l&apos;impression PDF, seules les variables de la table liée (« {primaryTableKey} ») et
          les variables système sont remplacées par les données de la ligne.
        </p>
      )}

      {catalog.tableBlocks.length > 0 && (
        <section>
          <Text variant="label" className="mb-2">
            Tableaux liste (pleine largeur HTML)
          </Text>
          <Text variant="muted" className="mb-2 text-xs">
            À placer dans un modèle « Liste » : remplace tout le tableau filtré à l&apos;export PDF.
          </Text>
          <ul className="space-y-1">
            {catalog.tableBlocks.map((b) => {
              const token = formatTableBlockToken(b.entityKey);
              return (
                <li key={b.entityKey}>
                  <button
                    type="button"
                    className="w-full rounded px-2 py-1.5 text-left font-mono text-sm hover:bg-surface-elevated"
                    onClick={() => void copy(token)}
                  >
                    {token}
                    <span className="ml-2 text-xs text-muted">
                      {b.label} ({b.entityKey})
                    </span>
                    {copied === token && (
                      <span className="ml-2 text-xs text-emerald-400">Copié</span>
                    )}
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      )}

      <section>
        <Text variant="label" className="mb-2">
          Système
        </Text>
        <ul className="space-y-1">
          {catalog.systemTable.fields.map((f) => {
            const token = formatVariableToken(catalog.systemTable.key, f.key);
            return (
              <li key={token}>
                <button
                  type="button"
                  className="w-full rounded px-2 py-1.5 text-left font-mono text-sm hover:bg-surface-elevated"
                  onClick={() => void copy(token)}
                >
                  {token}
                  <span className="ml-2 text-xs text-muted">{f.label}</span>
                  {copied === token && (
                    <span className="ml-2 text-xs text-emerald-400">Copié</span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      {catalog.tables.map((table) => (
        <section key={table.key}>
          <Text variant="label" className="mb-2">
            {table.label}{" "}
            <span className="font-mono text-xs font-normal text-muted">({table.key})</span>
          </Text>
          {table.fields.length === 0 ? (
            <p className="text-xs text-muted">Aucun attribut imprimable.</p>
          ) : (
            <ul className="space-y-1">
              {table.fields.map((f) => {
                const token = formatVariableToken(table.key, f.key);
                return (
                  <li key={token}>
                    <button
                      type="button"
                      className="w-full rounded px-2 py-1.5 text-left font-mono text-sm hover:bg-surface-elevated"
                      onClick={() => void copy(token)}
                    >
                      {token}
                      <span className="ml-2 text-xs text-muted">{f.label}</span>
                      {copied === token && (
                        <span className="ml-2 text-xs text-emerald-400">Copié</span>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      ))}
    </div>
  );
}
