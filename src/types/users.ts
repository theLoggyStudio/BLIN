export interface UserRow {
  id: string;
  nom: string;
  email: string;
  role: string;
  role_id: string;
  actif: boolean;
}

export interface RoleRow {
  id: string;
  nom: string;
}
