import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

interface HelpTipProps {
  children: ReactNode;
  /** Names the field this help belongs to. The accessible name becomes "Help: <label>"; without it, exactly "Help". */
  label?: string;
}

/** Horizontal shift that keeps a bubble with natural left edge `natural` and width `width` inside the viewport. */
export function clampShift(natural: number, width: number, viewport: number, margin = 8): number {
  if (natural < margin) return margin - natural;
  if (natural + width > viewport - margin) return Math.min(0, viewport - margin - width - natural);
  return 0;
}

/**
 * A toggletip. The trigger is a "?" button whose accessible name is "Help" (or "Help: <label>" when a label is given); it is
 * rendered next to a field, never inside its label, so it cannot change the field's name.
 * The bubble is always mounted (hidden while closed) and linked with aria-describedby. It opens on
 * hover, focus or click; Escape or a second click closes it.
 */
export function HelpTip({ children, label }: HelpTipProps) {
  const id = useId();
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [shift, setShift] = useState(0);
  const root = useRef<HTMLSpanElement>(null);
  const bubble = useRef<HTMLSpanElement>(null);
  const open = pinned || (!dismissed && (hover || focus));
  const close = useCallback(() => {
    setPinned(false);
    setHover(false);
    setDismissed(true);
  }, []);
  function activate() {
    document.dispatchEvent(new CustomEvent("navzeroth-help-activate", { detail: id }));
    setDismissed(false);
  }
  // Hover and keyboard input share one active bubble within this document.
  // Dismissing a competing bubble never changes its trigger's actual focus.
  useEffect(() => {
    const otherHelp = (event: Event) => {
      if (event instanceof CustomEvent && event.detail !== id) close();
    };
    document.addEventListener("navzeroth-help-activate", otherHelp);
    return () => document.removeEventListener("navzeroth-help-activate", otherHelp);
  }, [id, close]);


  useEffect(() => {
    if (!pinned) return;
    const away = (event: PointerEvent) => {
      if (root.current && event.target instanceof Node && !root.current.contains(event.target)) setPinned(false);
    };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [pinned]);

  // Escape dismisses the bubble wherever focus is, so hover-only help can be closed (WCAG 1.4.13).
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        close();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, close]);

  // Keep the bubble inside the viewport (320 px wide windows, 200 % text), and again after a resize.
  const shiftNow = useRef(0);
  const measure = useCallback(() => {
    const box = bubble.current?.getBoundingClientRect();
    if (box === undefined || box.width === 0) return; // not laid out (tests); nothing to measure
    const next = clampShift(box.left - shiftNow.current, box.width, document.documentElement.clientWidth);
    shiftNow.current = next;
    setShift(next);
  }, []);
  useLayoutEffect(() => {
    if (!open) {
      shiftNow.current = 0;
      setShift(0);
      return;
    }
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [open, measure]);

  return (
    <span
      className="ui-helptip"
      ref={root}
      onMouseEnter={() => {
        activate();
        setHover(true);
      }}
      onMouseLeave={() => setHover(false)}
    >
      {/* A span with role=button, not a <button>: a surrounding <fieldset disabled> must not make help unreachable. */}
      <span
        role="button"
        tabIndex={0}
        className="ui-helptip__trigger"
        aria-label={label === undefined || label === "" ? "Help" : `Help: ${label}`}
        aria-expanded={open}
        aria-controls={id}
        aria-describedby={id}
        onFocus={() => {
          activate();
          setFocus(true);
        }}
        onBlur={() => setFocus(false)}
        onClick={() => { if (pinned) close(); else { activate(); setPinned(true); } }}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            if (pinned) close();
            else { activate(); setPinned(true); }
          }
        }}
      >
        <span aria-hidden="true">?</span>
      </span>
      {/* Always mounted so aria-describedby resolves when the trigger gains focus; hidden while closed. */}
      <span id={id} className="ui-helptip__bubble" role="tooltip" hidden={!open} ref={bubble} style={shift === 0 ? undefined : { transform: `translateX(${shift}px)` }}>
        {children}
      </span>
    </span>
  );
}
