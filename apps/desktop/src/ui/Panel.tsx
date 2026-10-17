import { createContext, useContext, useId } from "react";
import type { ReactNode } from "react";

interface PanelProps {
  title: string;
  /** Heading level; pick the level that fits the page outline. */
  level?: 2 | 3 | 4;
  children: ReactNode;
}

const InsidePanel = createContext(false);

/**
 * Dark slate surface with a double bronze rule and an engraved heading. A Panel inside another
 * Panel is drawn flat (a top rule, no frame, no side padding) so nesting does not eat the
 * width of narrow windows or large text.
 */
export function Panel({ title, level = 2, children }: PanelProps) {
  const id = useId();
  const nested = useContext(InsidePanel);
  const Heading = `h${level}` as const;
  return (
    <section className={nested ? "ui-panel ui-panel--nested" : "ui-panel"} aria-labelledby={id}>
      <Heading id={id} className="ui-panel__title">
        {title}
      </Heading>
      <InsidePanel.Provider value={true}>
        <div className="ui-panel__body">{children}</div>
      </InsidePanel.Provider>
    </section>
  );
}
