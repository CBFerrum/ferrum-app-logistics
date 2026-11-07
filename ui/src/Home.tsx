// Today's trips for dispatchers: each with its load, distance, how many stops are done and, once
// completed, its cost; then the vehicles whose service is due. A trip or a vehicle opens its
// record; drivers have their own page of stops, and Fleet Costs gives each vehicle's over a range.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath, formatDate, formatDecimal, formatValue, todayIso, t as tr, isolate } from "@ferrum/client";
import { Badge, ErrorState, PageLoading, statusTone, useShellTitle } from "@ferrum/kit";

export interface Row {
  id: string;
  [k: string]: unknown;
}

export const rows = (entity: string, filters: unknown[], extra: Record<string, string | number> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

/** Trips read whole: a list's rows come without their stops. ponytail: one request a trip, a day's
 * worth; a summary endpoint when a day has hundreds. */
export const whole = (list: Row[]) => Promise.all(list.map((t) => api.get<Row>(docPath("Trip", t.id))));

/** A trip's stops: how many are done (delivered, collected or failed) of how many. */
export function progress(trip: Row): { done: number; of: number } {
  const stops = Array.isArray(trip.stops) ? (trip.stops as Row[]) : [];
  return { done: stops.filter((s) => (s.status ?? "Pending") !== "Pending").length, of: stops.length };
}

export function Home() {
  useShellTitle(tr("Logistics"));
  const day = todayIso();
  const q = useQuery({ queryKey: ["logistics", "trips", day], queryFn: async () => whole(await rows("Trip", [["date", "eq", day]], { sort: "number asc" })), refetchInterval: 30_000 });
  const due = useQuery({ queryKey: ["logistics", "service-due"], queryFn: () => rows("Vehicle", [["service_due", "eq", true]], { sort: "registration asc" }) });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>{tr("Today's trips")}</h2>
        <span className="row">
          <Link to="/drive">{tr("Your stops")}</Link>
          <Link to="/e/Trip/new">{tr("New trip")}</Link>
          <Link to="/r/logistics_fleet_costs">{tr("Fleet costs")}</Link>
        </span>
      </header>
      <div className="np-columns">
        {q.data.map((t) => {
          const p = progress(t);
          return (
            <Link key={t.id} className="card card-pad stack" to={`/e/Trip/${t.id}`}>
              <strong>{isolate(String(t.number ?? ""))}</strong>
              <span className="faint">
                {tr("{value} kg", { value: formatDecimal(t.total_weight ?? 0) })}{t.distance_km ? tr(" · {value} km", { value: formatDecimal(t.distance_km) }) : ""}
              </span>
              <span>{tr("{done} of {of} stops done", { done: p.done, of: p.of })}</span>
              {t.cost != null && <span className="faint">{tr("Cost {value}", { value: formatValue({ type: "currency" }, t.cost) })}</span>}
              <Badge tone={statusTone(String(t.status))}>{isolate(String(t.status))}</Badge>
            </Link>
          );
        })}
      </div>
      {!!due.data?.length && (
        <section className="stack" aria-label={tr("Service due")}>
          <h2>{tr("Service due")}</h2>
          <ul className="stack">
            {due.data.map((v) => (
              <li key={v.id}>
                <Link to={`/e/Vehicle/${v.id}`}>{isolate(String(v.registration ?? ""))}</Link>
                <span className="faint">{v.next_service_on ? tr(" · by {value}", { value: formatDate(v.next_service_on) }) : ""}{v.odometer_km != null ? tr(" · {value} km", { value: formatDecimal(v.odometer_km, 0) }) : ""}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
