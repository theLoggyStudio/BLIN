import { FileDown, FileUp, Plus, RefreshCw } from "lucide-react";
import { Button } from "@/items/Button";
import { Text } from "@/items/Text";
import { Guard } from "@/components/Guard";
import type { Privilege } from "@/types/auth";
import type { ListLayout, ScreenPrivileges } from "@/types/screen";

interface ScreenHeaderProps {
  layout: ListLayout;
  privileges: ScreenPrivileges;
  onCreate?: () => void;
  onRefresh?: () => void;
  onPrintListPdf?: () => void;
  onImportCsv?: () => void;
  onExportCsv?: () => void;
  loading?: boolean;
}

export function ScreenHeader({
  layout,
  privileges,
  onCreate,
  onRefresh,
  onPrintListPdf,
  onImportCsv,
  onExportCsv,
  loading,
}: ScreenHeaderProps) {
  const actions = layout.actions ?? [];

  return (
    <div className="flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between mb-6">
      <div>
        <Text variant="title" as="h1" className="screen-title-gradient text-3xl">
          {layout.title}
        </Text>
        {layout.subtitle && (
          <div className="mt-2 rounded-lg bg-card px-4 py-2 text-sm text-muted border border-border">
            {layout.subtitle}
          </div>
        )}
      </div>
      <div className="flex flex-wrap gap-2 shrink-0">
        {actions.includes("export") && onExportCsv && privileges.export && (
          <Guard privilege={privileges.export as Privilege}>
            <Button variant="secondary" size="sm" onClick={onExportCsv} disabled={loading}>
              <FileDown className="h-4 w-4" />
              Exporter
            </Button>
          </Guard>
        )}
        {actions.includes("import") && onImportCsv && privileges.import && (
          <Guard privilege={privileges.import as Privilege}>
            <Button variant="secondary" size="sm" onClick={onImportCsv} disabled={loading}>
              <FileUp className="h-4 w-4" />
              Importer
            </Button>
          </Guard>
        )}
        {onPrintListPdf && (
          <Guard privilege={privileges.view}>
            <Button variant="secondary" size="sm" onClick={onPrintListPdf} disabled={loading}>
              <FileDown className="h-4 w-4" />
              PDF liste
            </Button>
          </Guard>
        )}
        {actions.includes("refresh") && onRefresh && (
          <Button variant="ghost" size="sm" onClick={onRefresh} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            Actualiser
          </Button>
        )}
        {actions.includes("create") && onCreate && (
          <Guard privilege={privileges.create as Privilege}>
            <Button size="sm" onClick={onCreate}>
              <Plus className="h-4 w-4" />
              Nouveau
            </Button>
          </Guard>
        )}
      </div>
    </div>
  );
}
