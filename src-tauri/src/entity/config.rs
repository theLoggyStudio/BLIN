use super::attr_types::is_reserved_attribute;
use super::compteur::{self, is_compteur_attr};
use super::registry::{EntityAttribute, EntityDef};
use super::schema::{attr_column, table_name};
use crate::dda::config::{
    FieldDef, FieldFormMeta, FieldListMeta, FieldOption, FormLayout, FormsLayout, ListLayout,
    PrintMeta, ScreenConfigFile, ScreenLayout, ScreenMeta, ScreenPrivileges, StorageMeta,
    VisibleWhen,
};
use super::stock::STOCK_ENTITY_KEY;

const TACHE_ENTITY_KEY: &str = "tache";

fn map_field_type(attr_type: &str) -> String {
    match attr_type {
        "entity" => "entity_ref".into(),
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
    let hint = "Rempli automatiquement à l'enregistrement (non modifiable).";
    let mk = |key: String, column: String, label: String, field_type: &str, list: bool| FieldDef {
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
            read_only: Some(true),
            auto_generated: Some(true),
            storage_folder: None,
            max_files: None,
            accept: None,
            ref_entity: None,
            relation_exclusive_parent: None,
        }),
        visible_when: None,
        validation: None,
    };
    vec![
        mk(
            compteur::column_libelle(attr),
            compteur::column_libelle(attr),
            format!("{root} — Libellé"),
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
    let column = attr_column(attr);
    let label = attr
        .label
        .clone()
        .unwrap_or_else(|| attr.nom.clone());
    let is_entity_ref = attr.attr_type == "entity";
    let is_photo = attr.attr_type == "photo";
    let field_type = map_field_type(&attr.attr_type);

    FieldDef {
        key: attr.nom.clone(),
        column: column.clone(),
        field_type,
        label,
        required: attr.required,
        default: attr.default.clone(),
        options: build_options(attr),
        list: Some(FieldListMeta {
            enabled: list_enabled && !is_entity_ref && !is_photo,
            sortable: list_enabled && !is_entity_ref && !is_photo,
        }),
        filter: None,
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
            ref_entity: if is_entity_ref {
                attr.r#ref
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_lowercase().replace(' ', "_"))
            } else {
                None
            },
            relation_exclusive_parent: if is_entity_ref {
                Some(true)
            } else {
                None
            },
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

fn apply_tache_screen_fields(fields: &mut [FieldDef]) {
    for f in fields.iter_mut() {
        if matches!(
            f.key.as_str(),
            "entite_a_valider" | "enregistrement_id" | "role_validateur"
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

pub fn build_screen_config(ent: &EntityDef) -> ScreenConfigFile {
    let label = ent.label.clone().unwrap_or_else(|| ent.nom.clone());
    let label_plural = format!("{label}s");
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
            label: "Créé le".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: !relations_mode,
                sortable: !relations_mode,
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
        } else {
            fields.push(build_field(attr, list_on));
        }
    }

    if relations_mode {
        fields.push(FieldDef {
            key: "_detail".into(),
            column: "_detail".into(),
            field_type: "detail_link".into(),
            label: "Détail".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: true,
                sortable: false,
            }),
            filter: None,
            form: None,
            visible_when: None,
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
    let row_click = if relations_mode {
        Some("detail".into())
    } else {
        Some("edit".into())
    };

    let mut list_layout = ListLayout {
        title: label.clone(),
        subtitle,
        actions: vec!["refresh".into(), "create".into()],
        row_click,
    };
    if ent.nom == STOCK_ENTITY_KEY {
        apply_stock_screen_fields(&mut fields, &mut list_layout);
    }
    if ent.nom == TACHE_ENTITY_KEY {
        apply_tache_screen_fields(&mut fields);
    }

    ScreenConfigFile {
        screen: ScreenMeta {
            key: ent.nom.clone(),
            label: label.clone(),
            label_plural: Some(label_plural),
            icon: Some("building".into()),
            route: format!("/entite/{}", ent.nom),
            system: false,
            ai_editable: false,
            table: table_name(&ent.nom),
            primary_key: pk,
            label_field: label_field.clone(),
            default_order_by: Some(label_field),
            privileges: ScreenPrivileges {
                view: format!("{priv_base}:voir"),
                create: format!("{priv_base}:creer"),
                update: format!("{priv_base}:modifier"),
                delete: format!("{priv_base}:supprimer"),
                import: None,
                export: None,
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
