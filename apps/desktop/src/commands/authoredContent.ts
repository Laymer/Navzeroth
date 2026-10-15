export interface AuthoredSelection { revision: string; id: string }
export interface AuthoredCatalog {
  revision: string | null;
  catalog_sha256?: string | null;
  records: { id: string; label: string; evidence: string[] }[];
  total: number;
  next_offset: number | null;
}
