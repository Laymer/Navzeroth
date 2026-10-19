import { useEffect, useState } from "react";
import type { CatalogSubject } from "../commands/catalogPages";
import { originKey, type OriginClient, type OriginMark, type OriginRef } from "../commands/world";

/** The most records one lookup carries; every catalog page is at most this large. */
export const ORIGIN_BATCH = 200;
export type Origins = ReadonlyMap<string, OriginMark[]>;
const EMPTY: Origins = new Map();

/**
 * Origin marks for the records a view currently shows, computed by the backend from
 * each record's evidence, source editions and confidence. Lookups bind the exact
 * catalog revision. A failed or unavailable lookup shows no markers rather than a guess.
 */
export function useRecordOrigins(client: Partial<OriginClient> | undefined, subject: CatalogSubject | null, revision: string | null | undefined, refs: readonly OriginRef[]): Origins {
  const lookup = client?.getRecordOrigins;
  const unique = [...new Map(refs.map((ref) => [originKey(ref), ref])).values()].slice(0, ORIGIN_BATCH);
  const binding = JSON.stringify([subject, revision ?? null, unique.map(originKey)]);
  const [state, setState] = useState<{ binding: string; origins: Origins }>({ binding: "", origins: EMPTY });
  useEffect(() => {
    if (!lookup || !subject || !revision || unique.length === 0) return;
    let live = true;
    void lookup(JSON.stringify({ subject, revision, records: unique })).then((result) => {
      if (!live || !result.ok || result.value.revision !== revision) return;
      const origins = new Map<string, OriginMark[]>();
      unique.forEach((ref, index) => { const marks = result.value.origins[index]; if (marks) origins.set(originKey(ref), marks); });
      setState({ binding, origins });
    });
    return () => { live = false; };
    // The binding string captures subject, revision and every requested key.
  }, [lookup, binding]);
  return state.binding === binding ? state.origins : EMPTY;
}
