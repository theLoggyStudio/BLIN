import type {
  FieldDef,
  ScreenConfigFile,
  ScreenRow,
  ValidationIssue,
  ValidationReport,
} from "@/types/screen";
import { parseImagesValue } from "./mediaUtils";
import { isFieldVisible } from "./screenUtils";

function emptyValue(value: unknown, field?: FieldDef): boolean {
  if (field?.type === "image") {
    return value == null || String(value).trim() === "";
  }
  if (field?.type === "images") {
    return parseImagesValue(value).length === 0;
  }
  if (value == null) return true;
  if (typeof value === "string") return value.trim() === "";
  return false;
}

function asString(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number") return String(value);
  return String(value);
}

function matchesWhen(
  when: { field: string; equals: string | number | boolean },
  values: ScreenRow,
): boolean {
  const cur = values[when.field];
  return String(cur ?? "") === String(when.equals);
}

function issue(
  field: FieldDef,
  level: "error" | "warning",
  code: string,
  message: string,
  fixHint?: string,
): ValidationIssue {
  return {
    field: field.key,
    label: field.label,
    level,
    code,
    message,
    fixHint,
  };
}

function validateOneField(
  field: FieldDef,
  values: ScreenRow,
): { errors: ValidationIssue[]; warnings: ValidationIssue[] } {
  const errors: ValidationIssue[] = [];
  const warnings: ValidationIssue[] = [];
  const value = values[field.key] ?? values[field.column];
  const rules = field.validation;
  const isRequired = Boolean(field.required || rules?.required);

  if (isRequired && emptyValue(value, field)) {
    errors.push(
      issue(
        field,
        "error",
        "required",
        rules?.requiredMessage ?? `« ${field.label} » est obligatoire.`,
        rules?.fixHint ?? field.form?.placeholder,
      ),
    );
    return { errors, warnings };
  }

  if (emptyValue(value, field)) {
    return { errors, warnings };
  }

  if (field.type === "images" && field.form?.maxFiles != null) {
    const count = parseImagesValue(value).length;
    if (count > field.form.maxFiles) {
      errors.push(
        issue(
          field,
          "error",
          "max_files",
          `Maximum ${field.form.maxFiles} photo(s) dans la galerie.`,
          rules?.fixHint,
        ),
      );
    }
  }

  if (field.type === "image" || field.type === "images") {
    return { errors, warnings };
  }

  const text = asString(value);

  if (rules) {
    if (rules.minLength != null && text.length < rules.minLength) {
      errors.push(
        issue(
          field,
          "error",
          "min_length",
          rules.minLengthMessage ??
            `« ${field.label} » : au moins ${rules.minLength} caractères.`,
          rules.fixHint,
        ),
      );
    }
    if (rules.maxLength != null && text.length > rules.maxLength) {
      errors.push(
        issue(
          field,
          "error",
          "max_length",
          rules.maxLengthMessage ??
            `« ${field.label} » : maximum ${rules.maxLength} caractères.`,
          rules.fixHint,
        ),
      );
    }
    if (field.type === "number") {
      const n = Number(value);
      if (Number.isNaN(n)) {
        errors.push(
          issue(
            field,
            "error",
            "not_a_number",
            `« ${field.label} » doit être un nombre.`,
            rules.fixHint,
          ),
        );
      } else {
        const min = rules.min ?? field.form?.min;
        const max = rules.max;
        if (min != null && n < min) {
          errors.push(
            issue(
              field,
              "error",
              "min",
              rules.minMessage ?? `« ${field.label} » doit être ≥ ${min}.`,
              rules.fixHint,
            ),
          );
        }
        if (max != null && n > max) {
          errors.push(
            issue(
              field,
              "error",
              "max",
              rules.maxMessage ?? `« ${field.label} » doit être ≤ ${max}.`,
              rules.fixHint,
            ),
          );
        }
      }
    }
    if (rules.pattern) {
      try {
        const re = new RegExp(rules.pattern);
        if (!re.test(text)) {
          errors.push(
            issue(
              field,
              "error",
              "pattern",
              rules.patternMessage ?? `« ${field.label} » : format invalide.`,
              rules.fixHint ?? `Format attendu : ${rules.pattern}`,
            ),
          );
        }
      } catch {
        /* pattern invalide ignoré côté front */
      }
    }
    for (const w of rules.warnings ?? []) {
      if (matchesWhen(w.when, values)) {
        warnings.push(
          issue(field, "warning", "conditional", w.message, w.fixHint),
        );
      }
    }
  }

  if (field.type === "select" && field.options?.length) {
    if (!field.options.some((o) => o.value === text)) {
      errors.push(
        issue(
          field,
          "error",
          "invalid_option",
          `« ${field.label} » : choisissez une valeur dans la liste.`,
          field.options.map((o) => o.value).join(", "),
        ),
      );
    }
  }

  return { errors, warnings };
}

function validateTacheVisibilityFields(values: ScreenRow): ValidationIssue[] {
  const vis = String(values.visibilite ?? "publique");
  if (vis !== "personnalisee") return [];
  const raw = String(values.roles_visibles ?? "").trim();
  const roles = raw
    .replace(/^,|,$/g, "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  if (roles.length > 0) return [];
  return [
    {
      field: "roles_visibles",
      label: "Rôles autorisés",
      level: "error",
      code: "required",
      message: "Sélectionnez au moins un rôle pour une visibilité personnalisée.",
    },
  ];
}

function validateTacheLinkFields(values: ScreenRow): ValidationIssue[] {
  const typeTache = String(values.type_tache ?? "generale");
  if (typeTache !== "validation" && typeTache !== "destockage") {
    return [];
  }
  const errors: ValidationIssue[] = [];
  const requiredLinks: { key: string; label: string }[] = [
    { key: "entite_a_valider", label: "Entité à valider" },
    { key: "enregistrement_id", label: "ID enregistrement" },
  ];
  for (const { key, label } of requiredLinks) {
    if (emptyValue(values[key])) {
      errors.push({
        field: key,
        label,
        level: "error",
        code: "required",
        message: `« ${label} » est obligatoire pour une tâche de type « ${typeTache} ».`,
      });
    }
  }
  if (typeTache === "validation" && emptyValue(values.role_validateur)) {
    errors.push({
      field: "role_validateur",
      label: "Rôle valideur",
      level: "error",
      code: "required",
      message: "« Rôle valideur » est obligatoire pour une tâche de validation.",
    });
  }
  return errors;
}

function validateStockPeremption(values: ScreenRow): ValidationIssue[] {
  const perishable =
    values.article_perissable === true ||
    values.article_perissable === 1 ||
    values.article_perissable === "1" ||
    values.article_perissable === "true";
  if (!perishable) return [];
  const date = values.date_peremption;
  if (date == null || String(date).trim() === "") {
    return [
      {
        field: "date_peremption",
        label: "Date de péremption",
        level: "error",
        code: "required",
        message:
          "Pour un article périssable, la date de péremption est obligatoire.",
      },
    ];
  }
  return [];
}

export function validateScreenForm(
  config: ScreenConfigFile,
  values: ScreenRow,
  options?: { filtersOnly?: boolean },
): ValidationReport {
  const errors: ValidationIssue[] = [];
  const warnings: ValidationIssue[] = [];

  if (!options?.filtersOnly && config.screen.key === "stock") {
    errors.push(...validateStockPeremption(values));
  }
  if (!options?.filtersOnly && config.screen.key === "tache") {
    errors.push(...validateTacheLinkFields(values));
    errors.push(...validateTacheVisibilityFields(values));
  }

  for (const field of config.fields) {
    if (field.type === "hidden") continue;
    if (options?.filtersOnly && !field.filter?.enabled) continue;
    if (!options?.filtersOnly && !isFieldVisible(field, values)) continue;

    const r = validateOneField(field, values);
    errors.push(...r.errors);
    warnings.push(...r.warnings);
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

export function parseValidationReportFromError(err: unknown): ValidationReport | null {
  const raw = String(err);
  try {
    const parsed = JSON.parse(raw) as ValidationReport;
    if (Array.isArray(parsed.errors)) return parsed;
  } catch {
    /* pas un rapport structuré */
  }
  return null;
}

export function issuesByField(
  issues: ValidationIssue[],
): Record<string, ValidationIssue> {
  const map: Record<string, ValidationIssue> = {};
  for (const i of issues) {
    map[i.field] = i;
  }
  return map;
}
