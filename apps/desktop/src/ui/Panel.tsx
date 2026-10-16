import { useId } from "react";
import type { ReactNode } from "react";

interface PanelProps {
  title: string;
  /** Heading level; pick the level that fits the page outline. */
  level?: 2 | 3 | 4;
  children: ReactNode;
}

/** Dark slate surface with a double bronze rule and an engraved heading. */
export function Panel({ title, level = 2, children }: PanelProps) {
  const id = useId();
  const Heading = `h${level}` as const;
  return (
    <section className="ui-panel" aria-labelledby={id}>
      <Heading id={id} className="ui-panel__title">
        {title}
      </Heading>
      <div className="ui-panel__body">{children}</div>
    </section>
  );
}
