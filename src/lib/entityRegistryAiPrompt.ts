import type { RoleRow } from "@/types/users";

/** Idées par défaut si l'utilisateur ne précise pas de domaine (tirage aléatoire). */
const ECOSYSTEM_IDEAS: { ecosysteme: string; slogan: string; domaine: string }[] = [
  {
    ecosysteme: "Atelier Pro v1.0",
    slogan: "Clients, devis, interventions et suivi",
    domaine: "artisan, maintenance, SAV",
  },
  {
    ecosysteme: "Gestion Scolaire v1.0",
    slogan: "Établissements, classes, notes et absences",
    domaine: "établissement scolaire ou formation",
  },
  {
    ecosysteme: "Cabinet Santé v1.0",
    slogan: "Patients, rendez-vous et dossiers",
    domaine: "cabinet médical ou paramédical",
  },
  {
    ecosysteme: "Logistique Entrepôt v1.0",
    slogan: "Stock, expéditions et fournisseurs",
    domaine: "entrepôt, e-commerce, supply chain",
  },
  {
    ecosysteme: "Association Sport v1.0",
    slogan: "Adhérents, séances et compétitions",
    domaine: "club sportif ou association",
  },
];

export interface EntityRegistryPromptOptions {
  currentEcosystem?: string;
  currentSlogan?: string;
  /** Domaine métier saisi par l'utilisateur — pilote tout le prompt. */
  domainHint: string;
  /** Rôles déjà créés dans l'app (Paramètres → Rôles) — injectés pour validator_role_ids. */
  roles?: RoleRow[];
}

function ecosystemNameFromDomain(domain: string): string {
  const d = domain.trim();
  if (!d) return "Mon écosystème v1.0";
  const short = d.length > 48 ? `${d.slice(0, 45)}…` : d;
  return `Gestion ${short} v1.0`;
}

export function proposeEcosystemNames(options: EntityRegistryPromptOptions): {
  ecosysteme: string;
  slogan: string;
  domaine: string;
} {
  const domaine = options.domainHint.trim();

  if (options.currentEcosystem?.trim()) {
    return {
      ecosysteme: options.currentEcosystem.trim(),
      slogan:
        options.currentSlogan?.trim() ||
        `Organisez votre activité — ${domaine}`,
      domaine,
    };
  }

  const match = ECOSYSTEM_IDEAS.find((i) =>
    domaine.toLowerCase().split(/\s+/).some((w) => w.length > 3 && i.domaine.includes(w)),
  );
  if (match) {
    return {
      ecosysteme: match.ecosysteme,
      slogan: options.currentSlogan?.trim() || match.slogan,
      domaine,
    };
  }

  return {
    ecosysteme: ecosystemNameFromDomain(domaine),
    slogan: options.currentSlogan?.trim() || `Pilotage simplifié — ${domaine}`,
    domaine,
  };
}

function buildRolesSection(roles: RoleRow[] | undefined): string {
  if (!roles?.length) {
    return `## Rôles existants dans cette installation

Aucun rôle trouvé en base. Si une entité a \`requires_validation: true\`, laisse \`validator_role_ids: []\` ou propose des IDs au format \`role-xxx\` — l'utilisateur devra créer les rôles dans **Paramètres → Rôles** puis ajuster le JSON.

`;
  }

  const rows = roles
    .map((role) => `| \`${role.id}\` | ${role.nom.replace(/\|/g, "\\|")} |`)
    .join("\n");

  return `## Rôles existants dans cette installation

Utilise **uniquement** les IDs du tableau ci-dessous dans \`validator_role_ids\` (entités à valider). Pour les tâches en visibilité \`personnalisee\`, les mêmes IDs vont dans \`roles_visibles\` (CSV \`,role-id,\`).

| ID (à copier dans le JSON) | Nom affiché |
|-----------------------------|-------------|
${rows}

**Règle** : si \`requires_validation: true\`, choisis un ou plusieurs rôles **métier** adaptés au domaine (ex. directeur, responsable qualité) — pas uniquement \`role-admin\` sauf si pertinent.

`;
}

/**
 * Prompt complet à copier vers ChatGPT, Claude, etc. pour générer un registry.json Blin.
 */
export function buildEntityRegistryAiPrompt(options: EntityRegistryPromptOptions): string {
  const proposal = proposeEcosystemNames(options);
  const rolesSection = buildRolesSection(options.roles);

  return `Tu es un architecte de données pour **Blin**, une application desktop (Tauri + SQLite) **agnostique du métier** : chaque utilisateur définit son propre domaine (santé, sport, atelier, école, association, etc.). Il n'y a **pas** d'écran figé ni de module immobilier : tout est décrit dans un fichier \`registry.json\`, puis l'app génère tables SQLite, formulaires, privilèges et aide IA.

---

## Domaine cible (à respecter strictement)

**Domaine métier demandé par l'utilisateur** : ${proposal.domaine}

Tout le JSON doit être pensé **uniquement** pour ce domaine : vocabulaire, entités, attributs, enums et liaisons \`entity\`. Ne pas réutiliser un modèle immobilier, scolaire ou autre si ce n'est pas le domaine ci-dessus.

---

${rolesSection}## Ta mission

Conçois un **registre d'entités métier complet** pour ce domaine, prêt à être collé dans **Paramètres → Entités → Vue JSON** puis enregistré.

**Nom d'écosystème proposé** : \`${proposal.ecosysteme}\` (ajuste si un intitulé plus parlant existe pour le domaine)  
**Slogan proposé** : \`${proposal.slogan}\`

Inclure **au minimum 5 entités** cohérentes entre elles pour **${proposal.domaine}**, avec des **liaisons entity** (\`type: "entity"\` + \`ref\`) là où c'est logique. Ajoute une entité **tache** (ou équivalent nommé pour le suivi) si validations ou rappels sont pertinents pour ce métier.

---

## Structure JSON attendue (racine)

\`\`\`json
{
  "ecosysteme": "Nom affiché dans toute l'app (sidebar, titre fenêtre)",
  "slogan": "Phrase courte sous le nom",
  "logo_url": "",
  "entities": [ /* tableau d'entités */ ]
}
\`\`\`

- \`logo_url\` : laisser \`""\` (le logo se charge par fichier dans l'app).
- Ne pas mettre de champ \`logo\` en base64 dans le JSON exporté.

---

## Structure de chaque entité (\`entities[]\`)

\`\`\`json
{
  "nom": "client",
  "label": "Client",
  "description": "Une phrase métier pour l'IA et les utilisateurs",
  "ai_suggestions": false,
  "requires_validation": false,
  "validator_role_ids": [],
  "is_session": false,
  "attributs": [ /* voir ci-dessous */ ]
}
\`\`\`

| Champ | Règle |
|--------|--------|
| \`nom\` | Clé technique **unique**, minuscules, \`snake_case\` (ex. \`client\`, \`intervention\`, \`session_sport\`). |
| \`label\` | Libellé français affiché (accents OK). |
| \`description\` | Optionnelle mais recommandée. |
| \`ai_suggestions\` | Voir **règle de visibilité** ci-dessous (l'app recalcule à l'enregistrement). |

---

## Règle de visibilité — suggestions IA (barre « Gérer … »)

**Objectif** : la barre de commande ne propose que des entrées **métier intuitives** ; les fiches techniques restent accessibles via les formulaires (liaisons + « Créer un nouveau »).

| \`ai_suggestions\` | Quand l'utiliser |
|--------------------|------------------|
| \`false\` | Fiches **techniques / référence** sans phrase dans la barre : \`users\`, référentiels bruts, paramètres. **Aucune** liaison \`entity\` requise. |
| \`true\` | **Uniquement** si le formulaire contient **au moins un** attribut \`type: "entity"\` dont \`ref\` pointe vers une entité avec \`ai_suggestions: false\`. |

**Exemples (domaine scolaire)** :
- \`users\` → \`ai_suggestions: false\` (comptes, rôles).
- \`professeur\` avec \`"info" → ref: "users"\` → \`ai_suggestions: true\` (« Gérer les Enseignant »).
- \`eleve\` avec liaison vers \`users\` → \`true\`.
- \`matiere\` **sans** liaison vers une entité en \`false\` → \`false\` (on gère les matières depuis les notes / classes, pas depuis la barre).
- \`classe\` qui ne lie que d'autres entités déjà « visibles » (\`professeur\`, \`ecole\` en \`true\`) → \`false\` ; \`ecole\` qui lie \`users\` en \`false\` → \`true\`.

**Entités système** : \`stock\` (auto), \`tache\` → toujours \`ai_suggestions: false\` (menu **Tâches** dédié, pas la barre de commande).

**À l'enregistrement**, Blin **recalcule** \`ai_suggestions\` selon cette règle — ne pas mettre \`true\` partout par défaut.

**Trigger knowledge (auto)** : à chaque sauvegarde du registre, Blin génère \`MASTER_entities_relations.txt\` et \`{nom}_entity_relations.txt\` (jointures entity ref) pour le chat « liste les … avec … ».
| \`requires_validation\` | Si \`true\`, **trigger système** : à chaque \`dda_create\` / création, une tâche \`validation\` privée par rôle valideur (entité \`tache\` obligatoire). L'entité doit avoir **au moins un** attribut \`required: true\`. |
| \`validator_role_ids\` | Tableau d'IDs de rôles — **obligatoire** si \`requires_validation\` est true. Utilise **uniquement** les IDs listés dans la section « Rôles existants » ci-dessus. |
| \`is_session\` | Si \`true\`, chaque enregistrement peut être la **session métier active** (sidebar). Les entités avec un attribut \`entity\` vers cette session sont **filtrées** et **préremplies** à la création. Ex. \`seance\`, \`journee\`, \`intervention\`. |

---

## Structure de chaque attribut (\`attributs[]\`)

\`\`\`json
{
  "nom": "libelle",
  "type": "string",
  "label": "Intitulé",
  "required": true,
  "ref": null,
  "default": null,
  "enum_options": null
}
\`\`\`

### Types autorisés

\`string\`, \`number\`, \`integer\`, \`float\`, \`boolean\`, \`date\`, \`datetime\`, \`time\`, \`email\`, \`photo\`, \`uuid\`, \`entity\`, \`stock\`, \`compteur\`

- **Stock** : \`"type": "stock"\` (alias import : \`quantite_stock\`) — **ne pas** utiliser \`type: "number"\` pour l'inventaire. Exemple :
\`\`\`json
{ "nom": "quantite_en_stock", "type": "stock", "label": "Quantité en stock", "required": false }
\`\`\`
  - Au save du registre : entité système \`stock\` ajoutée **automatiquement** (ne pas la mettre dans \`entities[]\`).
  - Sync bidirectionnelle : quantité sur la fiche métier ↔ ligne inventaire (menu **Stock**).
  - Inventaire : \`article_perissable\` (bool) ; si true → \`date_peremption\` (date) **obligatoire**.
  - Péremption ≤ 30 jours → tâche \`type_tache: "destockage"\` (enum tache avec \`destockage\`).
  - Déstockage physique : UI Stock (bouton) ou outil \`entity_stock_destock\` — pas un type d'attribut JSON.

- **Compteur** : \`"type": "compteur"\` — **ne pas** demander la saisie à l'utilisateur. Exemple :
\`\`\`json
{ "nom": "reference", "type": "compteur", "label": "Référence document", "required": false }
\`\`\`
  - À la **création** : remplit automatiquement \`reference_libelle\` (libellé de l'attribut), \`reference_jjmmaaaa\` (date du jour JJMMYYYY), \`reference_numero\` (incrément du jour, repart à 1 chaque jour).
  - Champs **visibles en lecture seule** dans le formulaire après enregistrement (aperçu vide à la création).

- **Enum** : \`"type": "enum[val1,val2,val3]"\` **ou** \`"type": "enum"\` + \`"enum_options": ["val1","val2"]\`
- **Liaison** : \`"type": "entity"\`, \`"ref": "nom_entite_cible"\` (déclarer la cible dans \`entities\`)
- **Photo** : \`"type": "photo"\` pour upload image
- **Date / heure** : utiliser \`date\` et \`time\` (pas \`datetime\` sauf besoin réel)

### Attribut \`id\`

Tu peux inclure \`{ "nom": "id", "type": "uuid", "required": true }\` — l'application l'ignore à l'enregistrement (ID système).

---

## Entité \`tache\` (recommandée si validations, rappels ou stock périssable)

- \`libelle\` (string, requis)
- \`heure_debut\` (time, requis)
- \`date_echeance\` (date, optionnel)
- \`type_tache\` : \`enum[validation,generale,destockage]\` — \`destockage\` = alerte sortie stock (module stock)
- \`visibilite\` : \`enum[publique,privee,personnalisee]\` — **publique** = tous (\`tache:voir\`) ; **privee** = rôle \`role_validateur\` uniquement ; **personnalisee** = rôles listés dans \`roles_visibles\` (CSV \`,role-id,\`)
- Tâches de validation auto : \`visibilite: privee\` + \`role_validateur\` renseigné
- \`statut\` : \`enum[a_faire,en_cours,terminee]\`
- \`priorite\` : \`enum[basse,normale,haute]\` (optionnel)
- \`entite_a_valider\` / \`enregistrement_id\` — **obligatoires** si \`type_tache\` = \`validation\` ou \`destockage\`
- \`role_validateur\` (string) — **obligatoire** si \`type_tache\` = \`validation\` (rempli automatiquement par l'app)

---

## Module stock dans le JSON (règles)

1. **Ne pas** déclarer l'entité \`{ "nom": "stock", ... }\` dans \`entities\` — générée par l'app.
2. Déclarer au moins un attribut \`"type": "stock"\` sur une entité métier (ex. fourniture, produit).
3. Si articles périssables : prévoir \`tache\` avec \`destockage\` dans l'enum \`type_tache\`.
4. \`ai_suggestions\` sur \`stock\` : N/A (entité auto, \`ai_suggestions: false\` implicite).

---

## Erreurs fréquentes à **NE PAS** commettre

1. JSON invalide (virgules en trop, commentaires \`//\` interdits).
2. Copier un autre métier (immobilier, ERP générique) au lieu de **${proposal.domaine}**.
3. Doublons de \`nom\` d'entité ou d'attribut.
4. \`nom\` avec espaces ou majuscules.
5. \`ref\` vers une entité non déclarée.
6. Clés \`screen\`, \`fields\`, \`form\` (réservées au DDA généré — pas dans le registre).
7. Types inventés (\`text\`, \`varchar\`, \`relation\`).
8. \`requires_validation: true\` sans \`validator_role_ids\` non vide, ou sans aucun attribut \`required: true\`.
9. Entité \`stock\` manuelle dans \`entities\` (doublon système).
10. Inventaire en \`number\` au lieu de \`stock\` sur un champ dédié.
11. \`ai_suggestions: true\` sur une entité **sans** liaison \`entity\` vers une fiche en \`ai_suggestions: false\` (pollue la barre de commande).
12. \`ai_suggestions: false\` sur une entité métier **pourtant** liée à \`users\` (ou autre référentiel technique) — l'utilisateur ne la trouvera pas dans la barre.

---

## Livrable attendu

Réponds **uniquement** avec un bloc JSON valide (pretty-printé), sans markdown autour — prêt à coller dans Blin.

Vérifie :
- [ ] JSON parseable
- [ ] Au moins 5 entités **spécifiques au domaine** : ${proposal.domaine}
- [ ] \`ecosysteme\` et \`slogan\` en français
- [ ] Cohérence des liaisons \`entity\`
- [ ] Au moins une entité en \`ai_suggestions: false\` (ex. \`users\`) et des entités « métier » en \`true\` **uniquement** via liaison \`entity\` vers elle(s)

Référence de forme (ne pas recopier le métier) : fichier exemple \`gestion-scolaire.registry.example.json\` dans le dépôt Blin — même structure, **contenu entièrement différent** adapté au domaine ci-dessus.
`;
}
