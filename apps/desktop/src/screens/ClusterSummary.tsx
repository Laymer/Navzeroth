import type { ClusteringMetadata } from "../commands/clustering";

export function ClusterSummary({ clustering }: { clustering: ClusteringMetadata | undefined }) {
  if (!clustering || clustering.schema_version !== 1) return null;
  return <div aria-label="Objective clustering">
    <p>Objectives at the same declared node can share inbound travel. Completion and service times remain additive; shared kills are not assumed.</p>
    <ul>{clustering.selected_clusters.map((cluster) => <li key={cluster.members.join("|")}>
      {cluster.members.join(", ")}: {cluster.inbound_ms / 60_000} minutes inbound travel;
      {" "}minimum participant total utility {cluster.utility}
    </li>)}</ul>
  </div>;
}
