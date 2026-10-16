export type Tone = "ok" | "warn" | "danger" | "info" | "neutral";

// Each tone has its own shape, so the state never depends on colour alone.
const SHAPE: Record<Tone, string> = { ok: "●", warn: "▲", danger: "■", info: "◆", neutral: "○" };

interface StatusBadgeProps {
  tone: Tone;
  children: string;
}

/** Text plus a shape. The shape is decorative; the text carries the meaning. */
export function StatusBadge({ tone, children }: StatusBadgeProps) {
  return (
    <span className={`ui-badge ui-badge--${tone}`}>
      <span className="ui-badge__shape" aria-hidden="true">
        {SHAPE[tone]}
      </span>
      {children}
    </span>
  );
}
