import type { ScreenRow } from "@/types/screen";

export const SIGNATURE_STATUS_SIGNED = "signe";
export const SIGNATURE_STATUS_UNSIGNED = "non_signe";
export const SIGNATURE_STATUS_REFUSED = "refuse";

export function isRecordSigned(row: ScreenRow): boolean {
  return String(row.statut_signature ?? "") === SIGNATURE_STATUS_SIGNED;
}

export function isRecordRefused(row: ScreenRow): boolean {
  return String(row.statut_signature ?? "") === SIGNATURE_STATUS_REFUSED;
}

export function hasSignatureWorkflow(row: ScreenRow): boolean {
  const status = String(row.statut_signature ?? "");
  return status === SIGNATURE_STATUS_SIGNED
    || status === SIGNATURE_STATUS_UNSIGNED
    || status === SIGNATURE_STATUS_REFUSED;
}

/** Seul le créateur peut modifier avant signature ; jamais après (signé ou refusé). */
export function canCreatorEditRecord(row: ScreenRow, userId: string | undefined): boolean {
  if (!hasSignatureWorkflow(row)) return true;
  if (isRecordSigned(row) || isRecordRefused(row)) return false;
  const creator = String(row.cree_par ?? "").trim();
  if (!creator) return true;
  return userId != null && creator === userId;
}

export function isSignatureRecordReadOnly(row: ScreenRow, userId: string | undefined): boolean {
  if (!hasSignatureWorkflow(row)) return false;
  return isRecordSigned(row) || isRecordRefused(row) || !canCreatorEditRecord(row, userId);
}
