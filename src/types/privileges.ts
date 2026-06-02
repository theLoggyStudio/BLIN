export interface RoleRow {
  id: string;
  nom: string;
}

export interface RoleWithPrivileges {
  id: string;
  nom: string;
  privileges: string[];
}
