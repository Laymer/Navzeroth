// Unknown, unsupported, restricted, not-applicable and known values stay
// distinct, and are labelled in text so colour never carries the meaning.
type Obs<T> =
  | { status: "known"; value: T }
  | { status: "unknown" | "unsupported" | "restricted" | "not_applicable"; value?: never };

const LABEL = {
  unknown: "Unknown (not observed)",
  unsupported: "Unsupported by this client",
  restricted: "Restricted (hidden by the game)",
  not_applicable: "Not applicable",
} as const;

export function describeObservation<T extends string | number | boolean | object>(
  obs: Obs<T>,
  format: (value: T) => string = (v) => String(v),
): string {
  return obs.status === "known" ? format(obs.value) : LABEL[obs.status];
}
