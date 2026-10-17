import type { ButtonHTMLAttributes, Ref } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className"> {
  variant?: ButtonVariant;
  /** React 19 passes ref as an ordinary prop; roving-focus lists use it. */
  ref?: Ref<HTMLButtonElement>;
}

/**
 * A themed button. `type` defaults to "button" so it never submits a form by accident. A toggle passes
 * `aria-pressed` (from ButtonHTMLAttributes); a pressed button also has a distinct style in every mode.
 */
export function Button({ variant = "secondary", type = "button", ...rest }: ButtonProps) {
  return <button {...rest} type={type} className={`ui-button ui-button--${variant}`} />;
}
