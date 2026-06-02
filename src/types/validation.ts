export type ValidationLevel = "error" | "warning";

export interface ValidationIssue {
  field: string;
  label: string;
  level: ValidationLevel;
  code: string;
  message: string;
  fixHint?: string | null;
}

export interface ValidationReport {
  valid: boolean;
  errors: ValidationIssue[];
  warnings: ValidationIssue[];
}

export type FieldMessages = Record<string, { error?: string; warning?: string; fixHint?: string }>;
