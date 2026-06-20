/** Type brut ou enum[val1,val2] en JSON importé. */
export type EntityAttributeType =
  | "string"
  | "number"
  | "boolean"
  | "integer"
  | "float"
  | "stock"
  | "compteur"
  | "matricule"
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
  /** Liaison multiple vers l'entité cible (saisie en tableau incrémentable). */
  relation_multiple?: boolean;
  /** Exclusif parent (one-to-one / one-to-many strict). */
  relation_exclusive_parent?: boolean;
  /** Champ numérique source (entité porteuse de la liaison) pour l'impact. */
  relation_impact_source?: string | null;
  /** Champ cible numérique sur l'entité fille (stock, nombre, compteur…). */
  relation_impact_target?: string | null;
  /** Action sur le champ cible : increment | decrement. */
  relation_impact_action?: "increment" | "decrement" | null;
  /** Reporter l'impact à la validation de l'entité englobante (hiérarchie). */
  relation_impact_defer?: boolean;
  default?: string | number | boolean | null;
  enum_options?: string[];
}

export interface EntityDef {
  nom: string;
  label?: string;
  description?: string;
  /** Afficher dans les suggestions IA du tableau de bord (défaut : true). */
  ai_suggestions?: boolean;
  /** Trigger système : tâches de signature auto à chaque création (une par rôle signataire). */
  requires_signature?: boolean;
  /** @deprecated Alias legacy */
  requires_validation?: boolean;
  /** Identifiants de rôles SQLite autorisés à signer (ex. role-admin, role-directeur). */
  signatory_role_ids?: string[];
  /** @deprecated Alias legacy */
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

export interface EntityAccessInfo {
  allowed: boolean;
  entity_key: string;
  entity_label: string;
  contact_role_names: string[];
}

export interface EntityCreateDraft {
  entity_key: string;
  entity_label: string;
  initial_data: Record<string, unknown>;
  assistant_message: string;
}

export interface RegistryEntityCreateDraft {
  initial_entity: EntityDef;
  assistant_message: string;
}

export interface RegistryCreateMatchResult {
  matched: boolean;
  allowed: boolean;
  draft?: RegistryEntityCreateDraft;
}

export interface RegistryEntityCreateAction {
  initial_entity: EntityDef;
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
  /** Valeur brute du regroupement (tri chronologique). */
  sort_key?: string;
}

export interface StatsCatalogField {
  key: string;
  column: string;
  label: string;
  fieldType: string;
  temporal: boolean;
}

export interface StatsCatalogAggregate {
  value: string;
  label: string;
  needsValueField: boolean;
}

/** Catalogue statistiques généré par `trigger_stats` (commande entity_stats_config). */
export interface StatsCatalog {
  screenKey: string;
  entityLabel: string;
  abscissaFields: StatsCatalogField[];
  valueFields: StatsCatalogField[];
  aggregates: StatsCatalogAggregate[];
  defaultAbscissa: string | null;
  defaultValueField: string | null;
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
  /** Attributs formatés « Libellé : valeur · … » */
  detail?: string;
}

export interface SignatoryContact {
  userId: string;
  nom: string;
  email: string;
  roleId: string;
  roleNom: string;
}

/** @deprecated Utiliser SignatoryContact */
export type ValidatorContact = SignatoryContact;

export interface RecordSignatureField {
  key: string;
  label: string;
  value: string;
}

export interface RecordSignatureDetail {
  entityKey: string;
  entityLabel: string;
  recordId: string;
  signed: boolean;
  rejected: boolean;
  canView: boolean;
  canSign: boolean;
  canReject: boolean;
  refusedBy?: string | null;
  refusalReason?: string | null;
  fields: RecordSignatureField[];
  signatoryContacts: SignatoryContact[];
}

/** @deprecated Utiliser RecordSignatureDetail */
export type RecordValidationDetail = RecordSignatureDetail;
