import { useState } from "react";
import { Alert } from "@/items/Alert";
import { Text } from "@/items/Text";
import {
  buildAttributCatalog,
  formatAttributToken,
  formatTableBlockToken,
  type EntityAttributCatalog,
} from "@/lib/print/templateAttributes";
import type { EntityDef } from "@/types/entity";

interface PrintAttributesTabProps {
  entities: EntityDef[];
  primaryTableKey?: string | null;
}

/** Référence des attributs {{table.champ}} disponibles dans les modèles HTML/CSS. */
export function PrintAttributesTab({ entities, primaryTableKey }: PrintAttributesTabProps) {
  const catalog: EntityAttributCatalog = buildAttributCatalog(entities);
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
        Syntaxe : <code className="text-secondary">{`{{nomTable.nomAttribut}}`}</code>. Dans HTML ou
        CSS, tapez <code className="text-secondary">{`{{`}</code> pour afficher les tables
        existantes, puis choisissez un attribut après le point.
      </Text>
      {copied && (
        <Alert
          variant="success"
          size="box"
          role="status"
          message={`Attribut ${copied} copié dans le presse-papiers.`}
        />
      )}
      {primaryTableKey && (
        <Alert
          variant="warning"
          size="box"
          message={`À l'impression PDF, seuls les attributs de la table liée (« ${primaryTableKey} ») et les attributs système sont remplacés par les données de la ligne.`}
        />
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
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      )}

      {catalog.systemTables.map((systemTable) => (
        <section key={systemTable.key}>
          <Text variant="label" className="mb-2">
            {systemTable.label}
          </Text>
          <ul className="space-y-1">
            {systemTable.fields.map((f) => {
              const token = formatAttributToken(systemTable.key, f.key);
              return (
                <li key={token}>
                  <button
                    type="button"
                    className="w-full rounded px-2 py-1.5 text-left font-mono text-sm hover:bg-surface-elevated"
                    onClick={() => void copy(token)}
                  >
                    {token}
                    <span className="ml-2 text-xs text-muted">{f.label}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      ))}

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
                const token = formatAttributToken(table.key, f.key);
                return (
                  <li key={token}>
                    <button
                      type="button"
                      className="w-full rounded px-2 py-1.5 text-left font-mono text-sm hover:bg-surface-elevated"
                      onClick={() => void copy(token)}
                    >
                      {token}
                      <span className="ml-2 text-xs text-muted">{f.label}</span>
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
