// A driver's page: today's dispatched trips that are theirs (from /api/app/logistics/drive, which
// finds them by the address they signed in with), each stop in order with its address and a link
// that opens its proof of delivery already filled in for that stop.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api } from "@ferrum/client";
import { Badge, EmptyState, ErrorState, PageLoading, statusTone, useShellTitle } from "@ferrum/kit";
import { type Row, today } from "./Home";

/** Where a stop's proof of delivery opens, filled in for it. */
export function proofLink(trip: string, stop: unknown, outcome: "Delivered" | "Not Home"): string {
  const q = new URLSearchParams({ trip, stop: String(stop), outcome });
  return `/e/ProofOfDelivery/new?${q.toString()}`;
}

export function Drive() {
  useShellTitle("Your stops");
  const day = today();
  const q = useQuery({
    queryKey: ["logistics", "drive", day],
    queryFn: ({ signal }) => api.get<{ trips: Row[] }>("/api/app/logistics/drive", { day }, signal).then((r) => r.trips),
    refetchInterval: 60_000,
  });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data.length) return <EmptyState title="No trip out for you today" icon="truck" />;
  return (
    <div className="np-page">
      {q.data.map((t) => (
        <section key={t.id} className="card card-pad stack" aria-label={String(t.number ?? "")}>
          <h2>{String(t.number ?? "")}</h2>
          <ol className="stack">
            {(Array.isArray(t.stops) ? (t.stops as Row[]) : []).map((s) => (
              <li key={String(s.id ?? s.sequence)} className="stack">
                <span>
                  <strong>{String(s.sequence)}.</strong> {String(s.address ?? "")} · {String(s.weight_kg ?? 0)} kg{" "}
                  <Badge tone={statusTone(String(s.status ?? "Pending"))}>{String(s.status ?? "Pending")}</Badge>
                </span>
                {(s.status ?? "Pending") === "Pending" && (
                  <span className="row">
                    <Link to={proofLink(t.id, s.sequence, "Delivered")}>Delivered</Link>
                    <Link to={proofLink(t.id, s.sequence, "Not Home")}>Not delivered</Link>
                  </span>
                )}
              </li>
            ))}
          </ol>
        </section>
      ))}
    </div>
  );
}
