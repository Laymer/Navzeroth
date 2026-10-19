import { StatusBadge } from "./StatusBadge";
import type { OriginMark } from "../commands/world";

/** Visible text for each non-Forever origin (ADR 0007 §3). Forever facts show nothing. */
export const ORIGIN_LABEL: Record<OriginMark, string> = {
  vanilla_1_12: "From 1.12 data",
  wow_classic_era: "From Classic Era data",
  inferred: "Inferred",
};
const ORDER: OriginMark[] = ["vanilla_1_12", "wow_classic_era", "inferred"];

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
        <StatusBadge key={mark} tone={mark === "inferred" ? "warn" : "info"}>
          {ORIGIN_LABEL[mark]}
        </StatusBadge>
      ))}
    </span>
  );
}
