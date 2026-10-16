import type { ButtonHTMLAttributes } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className"> {
  variant?: ButtonVariant;
}

/** A themed button. `type` defaults to "button" so it never submits a form by accident. */
export function Button({ variant = "secondary", type = "button", ...rest }: ButtonProps) {
  return <button {...rest} type={type} className={`ui-button ui-button--${variant}`} />;
}
