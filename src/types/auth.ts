export type Privilege =
  | "users:voir"
  | "users:modifier"
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
