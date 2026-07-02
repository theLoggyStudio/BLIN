use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenConfigFile {
    pub screen: ScreenMeta,
    pub layout: ScreenLayout,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenMeta {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub label_plural: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    pub route: String,
    #[serde(default)]
    pub system: bool,
    #[serde(default)]
    pub ai_editable: bool,
    pub table: String,
    pub primary_key: String,
    pub label_field: String,
    #[serde(default)]
    pub default_order_by: Option<String>,
    pub privileges: ScreenPrivileges,
    #[serde(default)]
    pub print: Option<PrintMeta>,
    #[serde(default)]
    pub storage: Option<StorageMeta>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenPrivileges {
    pub view: String,
    pub create: String,
    pub update: String,
    pub delete: String,
    #[serde(default)]
    pub import: Option<String>,
    #[serde(default)]
    pub export: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintMeta {
    #[serde(default)]
    pub enabled: bool,
    pub screen_key: String,
    #[serde(default)]
    pub single_object: bool,
    #[serde(default)]
    pub template_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageMeta {
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenLayout {
    pub list: ListLayout,
    #[serde(default)]
    pub forms: Option<FormsLayout>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLayout {
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub row_click: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormsLayout {
    #[serde(default)]
    pub create: Option<FormLayout>,
    #[serde(default)]
    pub edit: Option<FormLayout>,
    #[serde(default)]
    pub detail: Option<FormLayout>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormLayout {
    pub title: String,
    pub mode: String,
    #[serde(default)]
    pub submit_label: Option<String>,
    #[serde(default)]
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub key: String,
    pub column: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub label: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Vec<FieldOption>,
    #[serde(default)]
    pub list: Option<FieldListMeta>,
    #[serde(default)]
    pub filter: Option<FieldFilterMeta>,
    #[serde(default)]
    pub form: Option<FieldFormMeta>,
    #[serde(default)]
    pub visible_when: Option<VisibleWhen>,
    #[serde(default)]
    pub validation: Option<FieldValidation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldValidation {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub required_message: Option<String>,
    #[serde(default)]
    pub min_length: Option<u32>,
    #[serde(default)]
    pub max_length: Option<u32>,
    #[serde(default)]
    pub min_length_message: Option<String>,
    #[serde(default)]
    pub max_length_message: Option<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub min_message: Option<String>,
    #[serde(default)]
    pub max_message: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub pattern_message: Option<String>,
    #[serde(default)]
    pub one_of: Option<Vec<String>>,
    #[serde(default)]
    pub one_of_message: Option<String>,
    #[serde(default)]
    pub fix_hint: Option<String>,
    #[serde(default)]
    pub warnings: Vec<FieldWarningRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldWarningRule {
    pub when: VisibleWhen,
    pub message: String,
    #[serde(default)]
    pub fix_hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldListMeta {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub sortable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldFilterMeta {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub operator: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldFormMeta {
    #[serde(default)]
    pub col_span: Option<u8>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub read_only: Option<bool>,
    /// Affiché en création malgré read_only (ex. compteur auto).
    #[serde(default)]
    pub auto_generated: Option<bool>,
    #[serde(default)]
    pub storage_folder: Option<String>,
    #[serde(default)]
    pub max_files: Option<u32>,
    #[serde(default)]
    pub accept: Option<String>,
    /// Liaison vers une autre entité (`entity_embed` / `entity_ref` en UI).
    #[serde(default)]
    pub ref_entity: Option<String>,
    #[serde(default)]
    pub relation_exclusive_parent: Option<bool>,
    #[serde(default)]
    pub relation_multiple: Option<bool>,
    /// Groupe d'embarquement parent (champs dupliqués depuis entité fille).
    #[serde(default)]
    pub embed_parent: Option<String>,
    /// Base matricule (ex. CMD) — injectée dans le DDA pour l'aperçu formulaire.
    #[serde(default)]
    pub matricule_base: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleWhen {
    pub field: String,
    pub equals: serde_json::Value,
}

/// Colonne réelle SQLite (exclut champs UI : hidden, detail_link).
pub fn is_persisted_field(field: &FieldDef) -> bool {
    field.field_type != "hidden"
        && field.field_type != "detail_link"
        && field.field_type != "entity_embed"
        && field.field_type != "entity_embed_list"
        && field.field_type != "matricule"
}

impl ScreenConfigFile {
    pub fn persisted_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| is_persisted_field(f)).collect()
    }

    pub fn writable_columns(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| {
                is_persisted_field(f)
                    && f.column != "id"
                    && f.column != "created_at"
                    && !f.form.as_ref().and_then(|m| m.read_only).unwrap_or(false)
            })
            .collect()
    }

    pub fn list_columns(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| {
                f.list.as_ref().is_some_and(|l| l.enabled) && is_persisted_field(f)
            })
            .collect()
    }

    pub fn filter_fields(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| f.filter.as_ref().is_some_and(|x| x.enabled))
            .collect()
    }

    pub fn media_fields(&self) -> Vec<&FieldDef> {
        self.fields
            .iter()
            .filter(|f| f.field_type == "image" || f.field_type == "images")
            .collect()
    }
}
