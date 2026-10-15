import type { CommandClient } from './client';
export type CatalogCollection = 'items'|'quests'|'acquisitions'|'nodes'|'objectives'|'models'|'dungeons';
export interface CatalogPageInfo { revision: string; catalog_sha256: string; collection: CatalogCollection; offset: number; total: number; next_offset: number|null }
export type CatalogPages = Partial<Record<CatalogCollection,CatalogPageInfo>>;
export interface CatalogRecordPage extends CatalogPageInfo { records: unknown[] }
export type CatalogPageClient = Pick<CommandClient,'getCatalogPage'>;
export type CatalogSubject = {kind:'character';profile_id:string}|{kind:'party';party_id:string};
