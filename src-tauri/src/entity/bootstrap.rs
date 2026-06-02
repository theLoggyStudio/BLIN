//! Registre métier minimal au premier lancement (base / registre vides).

use std::path::Path;

use super::registry::{self, EntityRegistry};

const DEFAULT_REGISTRY_JSON: &str = r#"{
  "ecosysteme": "Blin",
  "slogan": "Organisez vos tâches au quotidien",
  "entities": [
    {
      "nom": "tache",
      "label": "Tâche",
      "description": "Tâches et rappels — création via le tableau de bord ou Loggy",
      "ai_suggestions": true,
      "attributs": [
        { "nom": "id", "type": "uuid", "required": true },
        { "nom": "libelle", "type": "string", "label": "Intitulé", "required": true },
        { "nom": "description", "type": "string", "label": "Description", "required": false },
        { "nom": "date_echeance", "type": "date", "label": "Date", "required": false },
        { "nom": "heure_debut", "type": "time", "label": "Heure", "required": true },
        {
          "nom": "statut",
          "type": "enum[a_faire,en_cours,terminee]",
          "label": "Statut",
          "required": false,
          "default": "a_faire"
        },
        {
          "nom": "priorite",
          "type": "enum[basse,normale,haute]",
          "label": "Priorité",
          "required": false,
          "default": "normale"
        },
        {
          "nom": "type_tache",
          "type": "enum[validation,generale,destockage]",
          "label": "Type",
          "required": false,
          "default": "generale"
        },
        {
          "nom": "visibilite",
          "type": "enum[publique,privee,personnalisee]",
          "label": "Visibilité",
          "required": false,
          "default": "publique"
        },
        {
          "nom": "roles_visibles",
          "type": "string",
          "label": "Rôles autorisés (personnalisé)",
          "required": false
        },
        { "nom": "entite_a_valider", "type": "string", "label": "Entité à valider", "required": false },
        { "nom": "enregistrement_id", "type": "string", "label": "ID enregistrement", "required": false },
        { "nom": "role_validateur", "type": "string", "label": "Rôle valideur", "required": false }
      ]
    }
  ]
}"#;

/// Écrit `entities/registry.json` si absent ou sans entité (ex. après reset des données).
pub fn ensure_default_registry(data_dir: &Path) -> Result<bool, String> {
    let current = registry::load(data_dir)?;
    if !current.entities.is_empty() {
        return Ok(false);
    }

    let registry: EntityRegistry =
        serde_json::from_str(DEFAULT_REGISTRY_JSON).map_err(|e| e.to_string())?;
    registry::save(data_dir, &registry)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn bootstrap_writes_tache_entity() {
        let tmp = std::env::temp_dir().join(format!("blin-bootstrap-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        assert!(ensure_default_registry(&tmp).unwrap());
        let reg = registry::load(&tmp).unwrap();
        assert_eq!(reg.entities.len(), 1);
        assert_eq!(reg.entities[0].nom, "tache");
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn bootstrap_skips_when_entities_exist() {
        let tmp = std::env::temp_dir().join(format!("blin-bootstrap-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(tmp.join("entities")).unwrap();
        fs::write(
            tmp.join("entities").join("registry.json"),
            r#"{"entities":[{"nom":"client","label":"Client","attributs":[]}]}"#,
        )
        .unwrap();
        assert!(!ensure_default_registry(&tmp).unwrap());
        let _ = fs::remove_dir_all(tmp);
    }
}
