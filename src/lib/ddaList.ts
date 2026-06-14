import { invoke } from "@tauri-apps/api/core";
import type { ScreenRow } from "@/types/screen";

export interface DdaListResult {
  rows: ScreenRow[];
  total: number;
}

/** Compte les lignes sans charger tout le jeu (LIMIT 1 côté serveur). */
export async function fetchDdaListCount(
  screenKey: string,
  filters: Record<string, string> = {},
): Promise<number> {
  const data = await invoke<DdaListResult>("dda_list", {
    payload: {
      screen_key: screenKey,
      filters,
      page: 0,
      page_size: 1,
    },
  });
  return data.total;
}

/** Page paginée — préférer à un appel sans page/page_size. */
export async function fetchDdaListPage(
  screenKey: string,
  options: {
    filters?: Record<string, string>;
    page?: number;
    pageSize?: number;
  } = {},
): Promise<DdaListResult> {
  return invoke<DdaListResult>("dda_list", {
    payload: {
      screen_key: screenKey,
      filters: options.filters ?? {},
      page: options.page ?? 0,
      page_size: options.pageSize ?? 10,
    },
  });
}
