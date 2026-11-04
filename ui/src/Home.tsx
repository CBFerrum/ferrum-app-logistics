// Today's trips for dispatchers: each with its vehicle, driver, load, and how many stops are done.
// A trip opens its record; drivers have their own page of stops.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath } from "@ferrum/client";
import { Badge, ErrorState, PageLoading, statusTone, useShellTitle } from "@ferrum/kit";

export interface Row {
  id: string;
  [k: string]: unknown;
}

export const rows = (entity: string, filters: unknown[], extra: Record<string, string | number> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

export const today = () => new Date().toISOString().slice(0, 10);

/** Trips read whole: a list's rows come without their stops. ponytail: one request a trip, a day's
 * worth; a summary endpoint when a day has hundreds. */
export const whole = (list: Row[]) => Promise.all(list.map((t) => api.get<Row>(docPath("Trip", t.id))));

/** A trip's stops: how many are done (delivered or failed) of how many. */
export function progress(trip: Row): { done: number; of: number } {
  const stops = Array.isArray(trip.stops) ? (trip.stops as Row[]) : [];
  return { done: stops.filter((s) => s.status === "Delivered" || s.status === "Failed").length, of: stops.length };
}

export function Home() {
  useShellTitle("Logistics");
  const day = today();
  const q = useQuery({ queryKey: ["logistics", "trips", day], queryFn: async () => whole(await rows("Trip", [["date", "eq", day]], { sort: "number asc" })), refetchInterval: 30_000 });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>Today's trips</h2>
        <span className="row">
          <Link to="/drive">Your stops</Link>
          <Link to="/e/Trip/new">New trip</Link>
        </span>
      </header>
      <div className="np-columns">
        {q.data.map((t) => {
          const p = progress(t);
          return (
            <Link key={t.id} className="card card-pad stack" to={`/e/Trip/${t.id}`}>
              <strong>{String(t.number ?? "")}</strong>
              <span className="faint">
                {String(t.total_weight ?? 0)} kg{t.distance_km ? ` · ${String(t.distance_km)} km` : ""}
              </span>
              <span>{`${p.done} of ${p.of} stops done`}</span>
              <Badge tone={statusTone(String(t.status))}>{String(t.status)}</Badge>
            </Link>
          );
        })}
      </div>
    </div>
  );
}
