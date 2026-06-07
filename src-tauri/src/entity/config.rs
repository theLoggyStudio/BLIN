use super::attr_types::is_reserved_attribute;
use super::compteur::{self, is_compteur_attr};
use super::embed;
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use super::schema::{attr_column, table_name};
use crate::dda::config::{
    FieldDef, FieldFilterMeta, FieldFormMeta, FieldListMeta, FieldOption, FormLayout, FormsLayout,
    ListLayout, PrintMeta, ScreenConfigFile, ScreenLayout, ScreenMeta, ScreenPrivileges,
    StorageMeta, VisibleWhen,
};
use super::stock::STOCK_ENTITY_KEY;

const TACHE_ENTITY_KEY: &str = "tache";

const PLACEHOLDER_DEFAULTS: &[&str] = &["nom", "qte", "adr"];

fn sanitize_attr_default(attr: &EntityAttribute) -> Option<serde_json::Value> {
    let v = attr.default.clone()?;
    if attr.attr_type == "string" {
        if let Some(s) = v.as_str() {
            if PLACEHOLDER_DEFAULTS.contains(&s) {
                return None;
            }
        }
    }
    Some(v)
}

fn map_field_type(attr_type: &str) -> String {
    match attr_type {
        "matricule" => "matricule".into(),
        "photo" | "image" => "image".into(),
        "enum" => "select".into(),
        "number" | "integer" | "float" => "number".into(),
        "boolean" | "bool" => "boolean".into(),
        "stock" => "stock".into(),
        "compteur" => "compteur".into(),
        "date" => "date".into(),
        "time" => "time".into(),
        "datetime" => "datetime".into(),
        "email" => "text".into(),
        _ => "text".into(),
    }
}

fn has_entity_refs(ent: &EntityDef) -> bool {
    ent.attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
        .any(|a| a.attr_type == "entity")
}

fn has_photo_fields(ent: &EntityDef) -> bool {
    ent.attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
        .any(|a| a.attr_type == "photo")
}

fn build_options(attr: &EntityAttribute) -> Vec<FieldOption> {
    if attr.attr_type == "enum" {
        return attr
            .enum_options
            .as_ref()
            .map(|opts| {
                opts.iter()
                    .map(|v| FieldOption {
                        value: v.clone(),
                        label: v.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
    vec![]
}

fn build_compteur_fields(attr: &EntityAttribute, list_enabled: bool) -> Vec<FieldDef> {
    let root = attr
        .label
        .clone()
        .unwrap_or_else(|| attr.nom.clone());
    let is_matricule = attr.attr_type == "matricule";
    let hint = if is_matricule {
        "Saisissez la partie matricule ; date (jjmmaaaa) + n° sont auto."
    } else {
        "Rempli automatiquement à l'enregistrement (non modifiable)."
    };
    let libelle_col = compteur::column_libelle(attr);
    let mk = |key: String, column: String, label: String, field_type: &str, list: bool| {
        let is_manual_part = is_matricule && key == libelle_col;
        FieldDef {
            key,
            column,
            field_type: field_type.into(),
            label,
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: list,
                sortable: list,
            }),
            filter: None,
            form: Some(FieldFormMeta {
                col_span: None,
                placeholder: None,
                min: None,
                step: None,
                read_only: Some(!is_manual_part),
                auto_generated: Some(!is_manual_part),
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: None,
                relation_exclusive_parent: None,
                relation_multiple: None,
                embed_parent: None,
            }),
            visible_when: None,
            validation: None,
        }
    };
    vec![
        mk(
            compteur::column_libelle(attr),
            compteur::column_libelle(attr),
            if is_matricule {
                format!("{root} — Matricule (manuel)")
            } else {
                format!("{root} — Libellé")
            },
            "text",
            false,
        ),
        mk(
            compteur::column_jjmmaaaa(attr),
            compteur::column_jjmmaaaa(attr),
            format!("{root} — Date (jjmmaaaa)"),
            "text",
            list_enabled,
        ),
        mk(
            compteur::column_numero(attr),
            compteur::column_numero(attr),
            format!("{root} — N°"),
            "number",
            list_enabled,
        ),
    ]
    .into_iter()
    .map(|mut f| {
        if let Some(form) = f.form.as_mut() {
            form.placeholder = Some(hint.into());
        }
        f
    })
    .collect()
}

fn build_field(attr: &EntityAttribute, list_enabled: bool) -> FieldDef {
    build_field_with_key_column(attr, &attr.nom, &attr_column(attr), list_enabled, None)
}

fn build_field_with_key_column(
    attr: &EntityAttribute,
    key: &str,
    column: &str,
    list_enabled: bool,
    embed_parent: Option<String>,
) -> FieldDef {
    let label = attr
        .label
        .clone()
        .unwrap_or_else(|| attr.nom.clone());
    let is_entity_ref = attr.attr_type == "entity";
    let is_photo = attr.attr_type == "photo";
    let field_type = map_field_type(&attr.attr_type);

    FieldDef {
        key: key.to_string(),
        column: column.to_string(),
        field_type,
        label,
        required: attr.required,
        default: sanitize_attr_default(attr),
        options: build_options(attr),
        list: Some(FieldListMeta {
            enabled: list_enabled && !is_entity_ref && !is_photo,
            sortable: list_enabled && !is_entity_ref && !is_photo,
        }),
        filter: crate::dda::filters::filter_meta_for_attribute(&attr.attr_type, list_enabled),
        form: Some(FieldFormMeta {
            col_span: None,
            placeholder: if attr.attr_type == "time" {
                Some("HH:MM".into())
            } else if attr.attr_type == "email" {
                Some("ex. nom@ecole.fr".into())
            } else {
                None
            },
            min: None,
            step: if matches!(attr.attr_type.as_str(), "number" | "integer" | "float") {
                Some(1.0)
            } else {
                None
            },
            read_only: None,
            auto_generated: None,
            storage_folder: if is_photo {
                Some(format!("{}/photos", attr.nom))
            } else {
                None
            },
            max_files: None,
            accept: if is_photo {
                Some("image/*".into())
            } else {
                None
            },
            ref_entity: None,
            relation_exclusive_parent: None,
            relation_multiple: None,
            embed_parent,
        }),
        visible_when: None,
        validation: if attr.attr_type == "email" {
            Some(crate::dda::config::FieldValidation {
                pattern: Some(r"^[^\s@]+@[^\s@]+\.[^\s@]+$".into()),
                pattern_message: Some("Adresse e-mail invalide.".into()),
                required: false,
                required_message: None,
                min_length: None,
                max_length: None,
                min_length_message: None,
                max_length_message: None,
                min: None,
                max: None,
                min_message: None,
                max_message: None,
                one_of: None,
                one_of_message: None,
                fix_hint: None,
                warnings: vec![],
            })
        } else {
            None
        },
    }
}

fn build_entity_embed_header(parent_attr: &EntityAttribute, child: &EntityDef) -> FieldDef {
    let label = parent_attr
        .label
        .clone()
        .unwrap_or_else(|| parent_attr.nom.clone());
    FieldDef {
        key: parent_attr.nom.clone(),
        column: parent_attr.nom.clone(),
        field_type: "entity_embed".into(),
        label,
        required: parent_attr.required,
        default: None,
        options: vec![],
        list: Some(FieldListMeta {
            enabled: false,
            sortable: false,
        }),
        filter: None,
        form: Some(FieldFormMeta {
            col_span: Some(2),
            placeholder: None,
            min: None,
            step: None,
            read_only: None,
            auto_generated: None,
            storage_folder: None,
            max_files: None,
            accept: None,
            ref_entity: Some(child.nom.clone()),
            relation_exclusive_parent: Some(true),
            relation_multiple: Some(false),
            embed_parent: None,
        }),
        visible_when: None,
        validation: None,
    }
}

fn build_entity_embed_list_field(parent_attr: &EntityAttribute, child: &EntityDef) -> FieldDef {
    let label = parent_attr
        .label
        .clone()
        .unwrap_or_else(|| parent_attr.nom.clone());
    FieldDef {
        key: parent_attr.nom.clone(),
        column: attr_column(parent_attr),
        field_type: "entity_embed_list".into(),
        label,
        required: parent_attr.required,
        default: None,
        options: vec![],
        list: Some(FieldListMeta {
            enabled: false,
            sortable: false,
        }),
        filter: None,
        form: Some(FieldFormMeta {
            col_span: Some(2),
            placeholder: None,
            min: None,
            step: None,
            read_only: None,
            auto_generated: None,
            storage_folder: None,
            max_files: None,
            accept: None,
            ref_entity: Some(child.nom.clone()),
            relation_exclusive_parent: Some(true),
            relation_multiple: Some(true),
            embed_parent: None,
        }),
        visible_when: None,
        validation: None,
    }
}

fn build_embedded_compteur_fields(
    parent_attr: &EntityAttribute,
    child_attr: &EntityAttribute,
    list_enabled: bool,
) -> Vec<FieldDef> {
    let base = embed::embedded_column_name(parent_attr, child_attr);
    let root = format!(
        "{} — {}",
        parent_attr.label.as_deref().unwrap_or(&parent_attr.nom),
        child_attr.label.as_deref().unwrap_or(&child_attr.nom)
    );
    let parent_key = parent_attr.nom.clone();
    [
        (format!("{base}_libelle"), format!("{root} — Libellé"), "text"),
        (format!("{base}_jjmmaaaa"), format!("{root} — Date"), "text"),
        (format!("{base}_numero"), format!("{root} — N°"), "number"),
    ]
    .into_iter()
    .map(|(col, label, field_type)| FieldDef {
        key: col.clone(),
        column: col,
        field_type: field_type.into(),
        label,
        required: false,
        default: None,
        options: vec![],
        list: Some(FieldListMeta {
            enabled: list_enabled,
            sortable: list_enabled,
        }),
        filter: None,
        form: Some(FieldFormMeta {
            col_span: None,
            placeholder: None,
            min: None,
            step: None,
            read_only: Some(true),
            auto_generated: Some(true),
            storage_folder: None,
            max_files: None,
            accept: None,
            ref_entity: None,
            relation_exclusive_parent: None,
            relation_multiple: None,
            embed_parent: Some(parent_key.clone()),
        }),
        visible_when: None,
        validation: None,
    })
    .collect()
}

fn build_embed_fields_for_entity_attr(
    parent_attr: &EntityAttribute,
    child: &EntityDef,
    list_enabled: bool,
) -> Vec<FieldDef> {
    if parent_attr.relation_multiple {
        return vec![build_entity_embed_list_field(parent_attr, child)];
    }
    let mut fields = vec![build_entity_embed_header(parent_attr, child)];
    for child_attr in embed::copyable_child_attributes(child) {
        if is_compteur_attr(child_attr) {
            fields.extend(build_embedded_compteur_fields(
                parent_attr,
                child_attr,
                list_enabled,
            ));
        } else {
            let column = embed::embedded_column_name(parent_attr, child_attr);
            let parent_label = parent_attr.label.as_deref().unwrap_or(&parent_attr.nom);
            let child_label = child_attr.label.as_deref().unwrap_or(&child_attr.nom);
            let label = format!("{parent_label} — {child_label}");
            fields.push(build_field_with_key_column(
                child_attr,
                &column,
                &column,
                false,
                Some(parent_attr.nom.clone()),
            ));
            if let Some(last) = fields.last_mut() {
                last.label = label;
            }
        }
    }
    fields
}

fn apply_tache_screen_fields(fields: &mut [FieldDef]) {
    for f in fields.iter_mut() {
        if matches!(
            f.key.as_str(),
            "entite_a_signer"
                | "entite_a_valider"
                | "enregistrement_id"
                | "role_signataire"
                | "role_validateur"
                | "utilisateur_cible"
        ) {
            if let Some(form) = f.form.as_mut() {
                form.read_only = Some(true);
            }
        }
        if f.key == super::tache_visibility::COL_ROLES_VISIBLES {
            f.visible_when = Some(VisibleWhen {
                field: super::tache_visibility::COL_VISIBILITE.into(),
                equals: serde_json::json!(super::tache_visibility::VIS_PERSONNALISEE),
            });
            if let Some(list) = f.list.as_mut() {
                list.enabled = false;
            }
        }
    }
}

fn apply_stock_screen_fields(fields: &mut [FieldDef], list: &mut ListLayout) {
    list.actions = vec!["refresh".into()];
    for f in fields.iter_mut() {
        if matches!(f.key.as_str(), "entite_source" | "enregistrement_id" | "libelle") {
            if let Some(form) = f.form.as_mut() {
                form.read_only = Some(true);
            }
        }
        if f.key == "date_peremption" {
            f.visible_when = Some(VisibleWhen {
                field: "article_perissable".into(),
                equals: serde_json::json!(true),
            });
        }
    }
}

fn first_listable_column(ent: &EntityDef) -> String {
    const PRIORITY: &[&str] = &["nom", "libelle", "titre", "reference", "intitule"];
    for key in PRIORITY {
        if let Some(attr) = ent
            .attributs
            .iter()
            .find(|a| !is_reserved_attribute(a) && a.nom == *key)
        {
            return attr_column(attr);
        }
    }
    for attr in ent.attributs.iter().filter(|a| !is_reserved_attribute(a)) {
        if is_compteur_attr(attr) {
            return compteur::column_numero(attr);
        }
    }
    ent.attributs
        .iter()
        .filter(|a| {
            !is_reserved_attribute(a)
                && a.attr_type != "entity"
                && a.attr_type != "photo"
                && !is_compteur_attr(a)
        })
        .map(|a| attr_column(a))
        .next()
        .unwrap_or_else(|| "id".to_string())
}

fn entity_display_label(ent: &EntityDef) -> String {
    ent.label
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| humanize_entity_nom(&ent.nom))
}

fn humanize_entity_nom(nom: &str) -> String {
    let s = nom.replace('_', " ");
    let mut chars = s.chars();
    match chars.next() {
        None => nom.to_string(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn pluralize_label(label: &str) -> String {
    let t = label.trim();
    if t.ends_with('s') || t.ends_with('x') || t.ends_with('z') {
        return t.to_string();
    }
    format!("{t}s")
}

pub fn build_screen_config(ent: &EntityDef, registry: &EntityRegistry) -> ScreenConfigFile {
    let label = entity_display_label(ent);
    let label_plural = pluralize_label(&label);
    let pk = "id".to_string();
    let relations_mode = has_entity_refs(ent);
    let label_field = first_listable_column(ent);
    let subtitle = ent
        .description
        .clone()
        .or_else(|| Some(format!("Gestion dynamique — entité « {} »", ent.nom)));

    let mut fields = vec![
        FieldDef {
            key: "id".into(),
            column: "id".into(),
            field_type: "hidden".into(),
            label: "ID".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: false,
                sortable: false,
            }),
            filter: None,
            form: None,
            visible_when: None,
            validation: None,
        },
        FieldDef {
            key: "created_at".into(),
            column: "created_at".into(),
            field_type: "datetime".into(),
            label: "Date et heure de création".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: true,
                sortable: true,
            }),
            filter: None,
            form: None,
            visible_when: None,
            validation: None,
        },
    ];

    for attr in ent
        .attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
    {
        let list_on = if relations_mode {
            attr_column(attr) == label_field || compteur::column_numero(attr) == label_field
        } else {
            attr.attr_type != "photo"
        };
        if is_compteur_attr(attr) {
            fields.extend(build_compteur_fields(attr, list_on));
        } else if attr.attr_type == "entity" {
            if let Some(child) = embed::resolve_child(registry, attr) {
                fields.extend(build_embed_fields_for_entity_attr(attr, child, list_on));
            }
        } else {
            fields.push(build_field(attr, list_on));
        }
    }

    if ent.requires_signature {
        use super::record_signature::{
            REFUSAL_REASON_COLUMN, REFUSED_BY_COLUMN, SIGNATURE_STATUS_COLUMN, SIGNED_BY_COLUMN,
            STATUS_NON_SIGNE, STATUS_REFUSE, STATUS_SIGNE,
        };
        fields.push(FieldDef {
            key: SIGNATURE_STATUS_COLUMN.into(),
            column: SIGNATURE_STATUS_COLUMN.into(),
            field_type: "select".into(),
            label: "Statut de signature".into(),
            required: false,
            default: Some(serde_json::json!(STATUS_NON_SIGNE)),
            options: vec![
                FieldOption {
                    value: STATUS_SIGNE.into(),
                    label: "Signé".into(),
                },
                FieldOption {
                    value: STATUS_NON_SIGNE.into(),
                    label: "Non signé".into(),
                },
                FieldOption {
                    value: STATUS_REFUSE.into(),
                    label: "Refusé".into(),
                },
            ],
            list: Some(FieldListMeta {
                enabled: true,
                sortable: true,
            }),
            filter: Some(FieldFilterMeta {
                enabled: true,
                operator: None,
            }),
            form: Some(FieldFormMeta {
                col_span: None,
                placeholder: None,
                min: None,
                step: None,
                read_only: Some(true),
                auto_generated: None,
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: None,
                relation_exclusive_parent: None,
                relation_multiple: None,
                embed_parent: None,
            }),
            visible_when: None,
            validation: None,
        });
        fields.push(FieldDef {
            key: SIGNED_BY_COLUMN.into(),
            column: SIGNED_BY_COLUMN.into(),
            field_type: "string".into(),
            label: "Signé par".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: true,
                sortable: true,
            }),
            filter: Some(FieldFilterMeta {
                enabled: false,
                operator: None,
            }),
            form: Some(FieldFormMeta {
                col_span: None,
                placeholder: None,
                min: None,
                step: None,
                read_only: Some(true),
                auto_generated: None,
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: None,
                relation_exclusive_parent: None,
                relation_multiple: None,
                embed_parent: None,
            }),
            visible_when: Some(VisibleWhen {
                field: SIGNATURE_STATUS_COLUMN.into(),
                equals: serde_json::json!(STATUS_SIGNE),
            }),
            validation: None,
        });
        fields.push(FieldDef {
            key: REFUSED_BY_COLUMN.into(),
            column: REFUSED_BY_COLUMN.into(),
            field_type: "string".into(),
            label: "Refusé par".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: true,
                sortable: true,
            }),
            filter: Some(FieldFilterMeta {
                enabled: false,
                operator: None,
            }),
            form: Some(FieldFormMeta {
                col_span: None,
                placeholder: None,
                min: None,
                step: None,
                read_only: Some(true),
                auto_generated: None,
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: None,
                relation_exclusive_parent: None,
                relation_multiple: None,
                embed_parent: None,
            }),
            visible_when: Some(VisibleWhen {
                field: SIGNATURE_STATUS_COLUMN.into(),
                equals: serde_json::json!(STATUS_REFUSE),
            }),
            validation: None,
        });
        fields.push(FieldDef {
            key: REFUSAL_REASON_COLUMN.into(),
            column: REFUSAL_REASON_COLUMN.into(),
            field_type: "string".into(),
            label: "Motif du refus".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: false,
                sortable: false,
            }),
            filter: Some(FieldFilterMeta {
                enabled: false,
                operator: None,
            }),
            form: Some(FieldFormMeta {
                col_span: Some(2),
                placeholder: None,
                min: None,
                step: None,
                read_only: Some(true),
                auto_generated: None,
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: None,
                relation_exclusive_parent: None,
                relation_multiple: None,
                embed_parent: None,
            }),
            visible_when: Some(VisibleWhen {
                field: SIGNATURE_STATUS_COLUMN.into(),
                equals: serde_json::json!(STATUS_REFUSE),
            }),
            validation: None,
        });
    }

    let storage = if has_photo_fields(ent) {
        Some(StorageMeta {
            folders: vec![format!("{}/photos", ent.nom)],
        })
    } else {
        None
    };

    let priv_base = ent.nom.clone();
    let row_click = Some("edit".into());

    let mut list_layout = ListLayout {
        title: label.clone(),
        subtitle,
        actions: vec![
            "refresh".into(),
            "create".into(),
            "import".into(),
            "export".into(),
        ],
        row_click,
    };
    if ent.nom == STOCK_ENTITY_KEY {
        apply_stock_screen_fields(&mut fields, &mut list_layout);
    }
    if ent.nom == TACHE_ENTITY_KEY {
        apply_tache_screen_fields(&mut fields);
        list_layout.actions.retain(|a| a != "import" && a != "export");
    }

    ScreenConfigFile {
        screen: ScreenMeta {
            key: ent.nom.clone(),
            label: label.clone(),
            label_plural: Some(label_plural),
            icon: Some("building".into()),
            route: format!("/entite/{}", ent.nom),
            system: false,
            ai_editable: ent.ai_suggestions,
            table: table_name(&ent.nom),
            primary_key: pk,
            label_field: label_field.clone(),
            default_order_by: Some("datetime(created_at) DESC".into()),
            privileges: ScreenPrivileges {
                view: format!("{priv_base}:voir"),
                create: format!("{priv_base}:creer"),
                update: format!("{priv_base}:modifier"),
                delete: format!("{priv_base}:supprimer"),
                import: Some(format!("{priv_base}:importer")),
                export: Some(format!("{priv_base}:exporter")),
            },
            print: Some(PrintMeta {
                enabled: true,
                screen_key: ent.nom.clone(),
                single_object: true,
                template_name: Some(format!("Fiche {label}")),
            }),
            storage,
        },
        layout: ScreenLayout {
            list: list_layout,
            forms: Some(FormsLayout {
                create: Some(FormLayout {
                    title: format!("Nouveau — {label}"),
                    mode: "modal".into(),
                    submit_label: Some("Créer".into()),
                    read_only: None,
                }),
                edit: Some(FormLayout {
                    title: format!("Modifier — {label}"),
                    mode: "modal".into(),
                    submit_label: Some("Enregistrer".into()),
                    read_only: None,
                }),
                detail: Some(FormLayout {
                    title: format!("Fiche — {label}"),
                    mode: "modal".into(),
                    submit_label: None,
                    read_only: Some(true),
                }),
            }),
        },
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::registry::{EntityAttribute, EntityDef, EntityRegistry};

    fn attr(nom: &str, t: &str, required: bool, ref_ent: Option<&str>) -> EntityAttribute {
        EntityAttribute {
            nom: nom.into(),
            attr_type: t.into(),
            label: None,
            required,
            r#ref: ref_ent.map(str::to_string),
            relation_multiple: false,
            relation_exclusive_parent: true,
            default: None,
            enum_options: None,
        }
    }

    #[test]
    fn embed_config_has_no_detail_link_column() {
        let client = EntityDef {
            nom: "client".into(),
            label: Some("Client".into()),
            description: None,
            ai_suggestions: true,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![attr("nom", "string", true, None)],
        };
        let articles = EntityDef {
            nom: "articles".into(),
            label: Some("Articles".into()),
            description: None,
            ai_suggestions: true,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![
                attr("nom", "string", true, None),
                attr("qte", "number", true, None),
            ],
        };
        let mut client_attr = attr("client", "entity", true, Some("client"));
        let mut article_attr = attr("article", "entity", true, Some("articles"));
        article_attr.relation_multiple = true;
        let da = EntityDef {
            nom: "demande_dachat".into(),
            label: Some("Demande d'achat".into()),
            description: None,
            ai_suggestions: true,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![client_attr, article_attr],
        };
        let registry = EntityRegistry {
            ecosysteme: None,
            slogan: None,
            logo_url: None,
            logo: None,
            entities: vec![client, articles, da.clone()],
        };
        let cfg = build_screen_config(&da, &registry);
        assert!(
            !cfg.fields.iter().any(|f| f.key == "_detail"),
            "le détail relationnel est géré par l'UI, pas une colonne SQLite"
        );
        assert!(
            cfg.fields.iter().any(|f| f.field_type == "entity_embed"),
            "liaison 1-1 en entity_embed"
        );
        assert!(
            cfg.fields.iter().any(|f| f.field_type == "entity_embed_list"),
            "liaison multiple en entity_embed_list"
        );
    }
}
