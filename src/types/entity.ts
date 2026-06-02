/** Type brut ou enum[val1,val2] en JSON importé. */
export type EntityAttributeType =
  | "string"
  | "number"
  | "boolean"
  | "integer"
  | "float"
  | "stock"
  | "compteur"
  | "datetime"
  | "date"
  | "time"
  | "email"
  | "photo"
  | "enum"
  | "uuid"
  | "entity"
  | string;

export interface EntityAttribute {
  nom: string;
  type: EntityAttributeType;
  label?: string;
  required?: boolean;
  ref?: string | null;
  default?: string | number | boolean | null;
  enum_options?: string[];
}

export interface EntityDef {
  nom: string;
  label?: string;
  description?: string;
  /** Afficher dans les suggestions IA du tableau de bord (défaut : true). */
  ai_suggestions?: boolean;
  /** Trigger système : tâches de validation auto à chaque création (une par rôle valideur). */
  requires_validation?: boolean;
  /** Identifiants de rôles SQLite (ex. role-admin, role-directeur). */
  validator_role_ids?: string[];
  /** Contexte métier : chaque enregistrement peut être la session active (filtrage des liaisons). */
  is_session?: boolean;
  attributs: EntityAttribute[];
}

export interface ActiveBusinessSession {
  entity_key: string;
  record_id: string;
  label?: string | null;
}

export interface SessionEntityInfo {
  key: string;
  label: string;
}

export interface SessionBinding {
  field_key: string;
  session_entity_key: string;
}

export interface EntityActiveSessionResponse {
  active: ActiveBusinessSession | null;
  session_entities: SessionEntityInfo[];
  binding?: SessionBinding | null;
}

export interface EntityRegistry {
  ecosysteme?: string;
  /** Slogan sous le titre (sidebar). */
  slogan?: string;
  /** Optionnel : URL (import JSON) — sinon fichier image → `logo` data-URI à l'enregistrement. */
  logo_url?: string;
  /** Data-URI base64 (fichier local ou chargé depuis le disque). */
  logo?: string;
  entities: EntityDef[];
}

export interface EntityRegistryResponse {
  ecosysteme?: string;
  slogan?: string;
  logo_url?: string;
  logo?: string;
  entities: EntityDef[];
  count: number;
  json: string;
}

export interface EntitySuggestion {
  key: string;
  label: string;
  phrase: string;
  privilege: string;
}

export interface EntityCreateDraft {
  entity_key: string;
  entity_label: string;
  initial_data: Record<string, unknown>;
  assistant_message: string;
}

export type StatAggregate = "count" | "sum" | "avg" | "max" | "min";

export interface EntityStatsPayload {
  entity_key: string;
  group_by: string;
  aggregate?: StatAggregate;
  value_field?: string | null;
  /** @deprecated Utiliser aggregate + value_field */
  metric?: string;
}

export interface EntityStatRow {
  label: string;
  value: number;
}

export interface RelationPanelField {
  key: string;
  label: string;
  value: string;
}

export interface RelationPanel {
  entityKey: string;
  label: string;
  primary: boolean;
  viaField?: string;
  fields: RelationPanelField[];
}

export interface RelationDetailResponse {
  panels: RelationPanel[];
}

export interface RelationSelectOption {
  value: string;
  label: string;
  /** `non_valide` | `valide` si l'entité cible exige une validation. */
  validationStatus?: string | null;
}

export interface RecordValidationField {
  key: string;
  label: string;
  value: string;
}

export interface RecordValidationDetail {
  entityKey: string;
  entityLabel: string;
  recordId: string;
  validated: boolean;
  canView: boolean;
  canValidate: boolean;
  fields: RecordValidationField[];
}
