import type { ButtonHTMLAttributes, Ref } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className"> {
  variant?: ButtonVariant;
  /** React 19 passes ref as an ordinary prop; roving-focus lists use it. */
  ref?: Ref<HTMLButtonElement>;
  /** Toggle state; a pressed button also shows a distinct style, not just the attribute. */
  "aria-pressed"?: boolean | "mixed";
}

/** A themed button. `type` defaults to "button" so it never submits a form by accident. */
export function Button({ variant = "secondary", type = "button", ...rest }: ButtonProps) {
  return <button {...rest} type={type} className={`ui-button ui-button--${variant}`} />;
}
