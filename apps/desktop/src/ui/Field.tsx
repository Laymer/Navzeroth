import { useId } from "react";
import type { ReactNode } from "react";
import { HelpTip } from "./HelpTip";

interface ControlProps {
  id: string;
  "aria-describedby": string | undefined;
  "aria-invalid": true | undefined;
}

interface FieldProps {
  label: string;
  /** Short always-visible hint under the control. */
  hint?: string;
  /** Longer explanation behind a Help toggletip: what it does, units, what empty means. */
  help?: ReactNode;
  error?: string;
  children: (control: ControlProps) => ReactNode;
}

/** A labelled control. The label names the control exactly; the Help tip sits beside it. */
export function Field({ label, hint, help, error, children }: FieldProps) {
  const id = useId();
  const hintId = `${id}-hint`;
  const errorId = `${id}-error`;
  const describedBy = [hint === undefined ? "" : hintId, error === undefined ? "" : errorId].join(" ").trim();
  return (
    <div className="ui-field">
      <div className="ui-field__head">
        <label htmlFor={id}>{label}</label>
        {help !== undefined && <HelpTip label={label}>{help}</HelpTip>}
      </div>
      {children({ id, "aria-describedby": describedBy === "" ? undefined : describedBy, "aria-invalid": error === undefined ? undefined : true })}
      {hint !== undefined && (
        <p id={hintId} className="ui-field__hint">
          {hint}
        </p>
      )}
      {error !== undefined && (
        <p id={errorId} className="ui-field__error">
          {error}
        </p>
      )}
    </div>
  );
}
