import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

interface HelpTipProps {
  children: ReactNode;
}

/**
 * A toggletip. The trigger is a "?" button whose accessible name is just "Help"; it is
 * rendered next to a field, never inside its label, so it cannot change the field's name.
 * The bubble is always mounted (hidden while closed) and linked with aria-describedby. It opens on
 * hover, focus or click; Escape or a second click closes it.
 */
export function HelpTip({ children }: HelpTipProps) {
  const id = useId();
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [shift, setShift] = useState(0);
  const root = useRef<HTMLSpanElement>(null);
  const bubble = useRef<HTMLSpanElement>(null);
  const open = pinned || (!dismissed && (hover || focus));

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
  }, [open]);

  // Keep the bubble inside the viewport (320 px wide windows, 200 % text).
  useLayoutEffect(() => {
    if (!open || bubble.current === null) {
      setShift(0);
      return;
    }
    const box = bubble.current.getBoundingClientRect();
    if (box.width === 0) return; // not laid out (tests); nothing to measure
    const margin = 8;
    const width = document.documentElement.clientWidth;
    const natural = box.left - shift;
    if (natural < margin) setShift(margin - natural);
    else if (natural + box.width > width - margin) setShift(Math.min(0, width - margin - box.width - natural));
    else setShift(0);
  }, [open]);

  function close() {
    setPinned(false);
    setHover(false);
    setDismissed(true);
  }

  return (
    <span
      className="ui-helptip"
      ref={root}
      onMouseEnter={() => {
        setDismissed(false);
        setHover(true);
      }}
      onMouseLeave={() => setHover(false)}
    >
      {/* A span with role=button, not a <button>: a surrounding <fieldset disabled> must not make help unreachable. */}
      <span
        role="button"
        tabIndex={0}
        className="ui-helptip__trigger"
        aria-label="Help"
        aria-expanded={open}
        aria-controls={id}
        aria-describedby={id}
        onFocus={() => {
          setDismissed(false);
          setFocus(true);
        }}
        onBlur={() => setFocus(false)}
        onClick={() => (pinned ? close() : setPinned(true))}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            if (pinned) close();
            else setPinned(true);
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
