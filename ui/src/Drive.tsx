// A driver's page: today's dispatched trips that are theirs (from /api/app/logistics/drive, which
// finds them by the address they signed in with), each stop in order with its address, whether it
// delivers or collects, its window, and links that open its proof of delivery already filled in for
// that stop; and a link to log fuel.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, todayIso } from "@ferrum/client";
import { Badge, EmptyState, ErrorState, PageLoading, statusTone, useShellTitle } from "@ferrum/kit";
import type { Row } from "./Home";

/** Where a stop's proof of delivery opens, filled in for it. */
export function proofLink(trip: string, stop: unknown, outcome: "Delivered" | "Collected" | "Not Home"): string {
  const q = new URLSearchParams({ trip, stop: String(stop), outcome });
  return `/e/ProofOfDelivery/new?${q.toString()}`;
}

/** A stop's window, as "09:00–11:00", "from 09:00" or "by 11:00"; empty without one. */
export function windowLabel(stop: Row): string {
  const [from, to] = [stop.window_from, stop.window_to].map((v) => (typeof v === "string" ? v.trim() : ""));
  if (from && to) return `${from}–${to}`;
  return from ? `from ${from}` : to ? `by ${to}` : "";
}

export function Drive() {
  useShellTitle("Your stops");
  const day = todayIso();
  const q = useQuery({
    queryKey: ["logistics", "drive", day],
    queryFn: ({ signal }) => api.get<{ trips: Row[] }>("/api/app/logistics/drive", { day }, signal).then((r) => r.trips),
    refetchInterval: 60_000,
  });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const fuel = <Link to="/e/FuelLog/new">Log fuel</Link>;
  if (!q.data.length)
    return (
      <EmptyState title="No trip out for you today" icon="truck">
        {fuel}
      </EmptyState>
    );
  return (
    <div className="np-page">
      <span className="row">{fuel}</span>
      {q.data.map((t) => (
        <section key={t.id} className="card card-pad stack" aria-label={String(t.number ?? "")}>
          <h2>{String(t.number ?? "")}</h2>
          <ol className="stack">
            {(Array.isArray(t.stops) ? (t.stops as Row[]) : []).map((s) => {
              const collect = s.kind === "Collection";
              const slot = windowLabel(s);
              return (
                <li key={String(s.id ?? s.sequence)} className="stack">
                  <span>
                    <strong>{String(s.sequence)}.</strong> {collect ? "Collect from " : ""}
                    {String(s.address ?? "")} · {String(s.weight_kg ?? 0)} kg{slot ? ` · ${slot}` : ""}{" "}
                    <Badge tone={statusTone(String(s.status ?? "Pending"))}>{String(s.status ?? "Pending")}</Badge>
                    {s.late === true && <Badge tone="amber">Late</Badge>}
                  </span>
                  {(s.status ?? "Pending") === "Pending" && (
                    <span className="row">
                      <Link to={proofLink(t.id, s.sequence, collect ? "Collected" : "Delivered")}>{collect ? "Collected" : "Delivered"}</Link>
                      <Link to={proofLink(t.id, s.sequence, "Not Home")}>{collect ? "Not collected" : "Not delivered"}</Link>
                    </span>
                  )}
                </li>
              );
            })}
          </ol>
        </section>
      ))}
    </div>
  );
}
