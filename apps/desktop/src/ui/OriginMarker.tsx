import { StatusBadge } from "./StatusBadge";
import type { OriginMark } from "../commands/world";

/** Visible historical attribution. Absence of a badge never proves live verification. */
export const ORIGIN_LABEL: Record<OriginMark, string> = {
  vanilla_1_12: "From 1.12 data",
  wow_classic_era: "From Classic Era data",
  inferred: "Inferred",
  wow_forever_70245: "From Forever 1.60.1.70245 (historical, unverified)",
};
const ORDER: OriginMark[] = ["vanilla_1_12", "wow_classic_era", "inferred", "wow_forever_70245"];

/**
 * Where a fact came from when it is not the Forever client: 1.12 (CMaNGOS) data,
 * Classic Era data, or a derived (inferred) value. Renders nothing for Forever-origin,
 * non-inferred or unresolved facts. The text carries the meaning; the shape is decorative.
 */
export function OriginMarker({ marks }: { marks: readonly OriginMark[] | null | undefined }) {
  const shown = ORDER.filter((mark) => marks?.includes(mark));
  if (shown.length === 0) return null;
  return (
    <span className="ui-origin" data-origin={shown.join(" ")}>
      {shown.map((mark) => (
        <StatusBadge key={mark} tone={mark === "inferred" || mark === "wow_forever_70245" ? "warn" : "info"}>
          {ORIGIN_LABEL[mark]}
        </StatusBadge>
      ))}
    </span>
  );
}
