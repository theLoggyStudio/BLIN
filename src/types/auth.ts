export type Privilege =
  | "users:voir"
  | "users:modifier"
  | "parametres:voir"
  | "parametres:assistant"
  | "parametres:compte"
  | "parametres:theme"
  | "parametres:impression"
  | "parametres:entites"
  | "parametres:entites:creer"
  | "parametres:roles"
  | "parametres:utilisateurs"
  | "documents:voir"
  | "documents:importer"
  | "documents:exporter"
  | "documents:supprimer"
  | "documents:modeles_voir"
  | "documents:modeles_gerer"
  | "ai:utiliser"
  | "directeur:confirmer"
  | "*";

export interface User {
  id: string;
  nom: string;
  email: string;
  role: string;
  privileges: Privilege[];
  must_change_password?: boolean;
}
