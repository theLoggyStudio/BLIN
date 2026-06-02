use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BienPieceNom {
    pub code: String,
    pub nom: String,
    #[serde(default)]
    pub type_piece: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BienChambreNom {
    pub code: String,
    pub nom: String,
    #[serde(default)]
    pub pieces: Vec<BienPieceNom>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BienEtageNom {
    pub numero: i32,
    pub libelle: String,
    #[serde(default)]
    pub chambres: Vec<BienChambreNom>,
    #[serde(default)]
    pub pieces_communes: Vec<BienPieceNom>,
}

pub fn parse_nomenclature_json(json: &str) -> Vec<BienEtageNom> {
    if json.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(json).unwrap_or_default()
}

pub fn serialize_nomenclature(etages: &[BienEtageNom]) -> String {
    serde_json::to_string(etages).unwrap_or_else(|_| "[]".to_string())
}

pub fn count_from_nomenclature(etages: &[BienEtageNom]) -> (i32, i32, i32) {
    let mut nb_chambres = 0i32;
    let mut nb_pieces = 0i32;
    for e in etages {
        nb_chambres += e.chambres.len() as i32;
        for ch in &e.chambres {
            nb_pieces += ch.pieces.len() as i32;
        }
        nb_pieces += e.pieces_communes.len() as i32;
    }
    (etages.len() as i32, nb_chambres, nb_pieces)
}

pub fn build_logement_cle(bien_reference: &str, etage_numero: i32, chambre_code: &str) -> String {
    let ref_slug = slug_part(bien_reference);
    let ch_slug = slug_part(chambre_code);
    format!("{ref_slug}-etage-{etage_numero}-{ch_slug}")
}

fn slug_part(s: &str) -> String {
    let lower = s.trim().to_lowercase();
    let mut out = String::new();
    let mut prev_dash = false;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn logement_cle_exists_in_bien(
    bien_reference: &str,
    nomenclature: &[BienEtageNom],
    cle: &str,
) -> bool {
    for etage in nomenclature {
        for ch in &etage.chambres {
            if build_logement_cle(bien_reference, etage.numero, &ch.code) == cle {
                return true;
            }
        }
    }
    false
}

pub fn normalize_bien_counts(
    type_bien: &str,
    nb_etages: Option<i32>,
    nb_chambres: Option<i32>,
    nb_pieces: Option<i32>,
    nomenclature: &[BienEtageNom],
) -> (Option<i32>, Option<i32>, Option<i32>, Vec<BienEtageNom>) {
    if type_bien == "hangar" {
        return (None, None, None, Vec::new());
    }
    if type_bien == "immeuble" && !nomenclature.is_empty() {
        let (ne, nc, np) = count_from_nomenclature(nomenclature);
        return (Some(ne), Some(nc), Some(np), nomenclature.to_vec());
    }
    let nom = if type_bien == "immeuble" {
        nomenclature.to_vec()
    } else {
        Vec::new()
    };
    (nb_etages, nb_chambres, nb_pieces, nom)
}
