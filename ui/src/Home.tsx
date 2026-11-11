// Today's trips for dispatchers: each with its load, distance, how many stops are done and, once
// completed, its cost; then the vehicles whose service is due. A trip or a vehicle opens its
// record; drivers have their own page of stops, and Fleet Costs gives each vehicle's over a range.

import { useQuery } from "@tanstack/react-query";
import { api, docPath, formatDate, formatDecimal, formatValue, todayIso, t as tr, isolate } from "@ferrum/client";
import { Badge, ButtonLink, HomeSkeleton, LoadFailed, PageHead, Queue, QueueGrid, QueueItem, StatePanel, StatusBadge, Tile, TileGrid } from "@ferrum/kit";

// Trip.status as stored (packages/logistics/entities).
export const TRIP_TONES = { Planned: "neutral", Dispatched: "blue", Completed: "green", Cancelled: "neutral" } as const;

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
  const day = todayIso();
  const q = useQuery({ queryKey: ["logistics", "trips", day], queryFn: async () => whole(await rows("Trip", [["date", "eq", day]], { sort: "number asc" })), refetchInterval: 30_000 });
  const due = useQuery({ queryKey: ["logistics", "service-due"], queryFn: () => rows("Vehicle", [["service_due", "eq", true]], { sort: "registration asc" }) });
  const head = (
    <PageHead
      title={tr("Today's trips")}
      actions={
        <>
          <ButtonLink to="/e/Trip/new" variant="primary" icon="plus">{tr("New trip")}</ButtonLink>
          <ButtonLink to="/drive">{tr("Your stops")}</ButtonLink>
          <ButtonLink to="/r/logistics_fleet_costs">{tr("Fleet costs")}</ButtonLink>
        </>
      }
    />
  );
  if (q.isPending) return <>{head}<HomeSkeleton queues={1} /></>;
  if (q.isError) return <>{head}<LoadFailed error={q.error} onRetry={() => void q.refetch()} /></>;
  return (
    <>
      {head}
      {q.data.length === 0 ? (
        <StatePanel kind="empty" title={tr("No trips today")}>{tr("Plan a trip for today and it shows here with its load and its stops.")}</StatePanel>
      ) : (
        <TileGrid label={tr("Today's trips")}>
          {q.data.map((t) => {
            const p = progress(t);
            const load = t.distance_km ? tr("Load: {weight} kg · {distance} km", { weight: formatDecimal(t.total_weight ?? 0), distance: formatDecimal(t.distance_km) }) : tr("Load: {value} kg", { value: formatDecimal(t.total_weight ?? 0) });
            return (
              <Tile
                key={t.id}
                to={`/e/Trip/${t.id}`}
                title={isolate(String(t.number ?? ""))}
                meta={
                  <>
                    <span>{isolate(load)}</span>
                    {p.of === 0 ? (
                      <span>{tr("No stops yet")}</span>
                    ) : (
                      <>
                        <span>{tr("{done} of {of} stops done", { done: p.done, of: p.of })}</span>
                        <span className="trip-meter" role="progressbar" aria-label={tr("Stops done")} aria-valuemin={0} aria-valuemax={p.of} aria-valuenow={p.done}>
                          <span style={{ inlineSize: `${(p.done / p.of) * 100}%` }} />
                        </span>
                      </>
                    )}
                    {t.cost != null && <span>{tr("Cost {value}", { value: formatValue({ type: "currency" }, t.cost) })}</span>}
                  </>
                }
                status={
                  <span className="trip-flags">
                    <StatusBadge value={t.status} tones={TRIP_TONES} label={isolate(String(t.status))} />
                    {t.weight_unknown === true && t.measured_weight_kg == null && t.status === "Planned" && <Badge tone="amber">{tr("Weight unknown: weigh it before dispatch")}</Badge>}
                  </span>
                }
              />
            );
          })}
        </TileGrid>
      )}
      {!!due.data?.length && (
        <QueueGrid className="service-due">
          <Queue title={tr("Service due")} count={due.data.length} tone="attention" more={{ to: "/e/Vehicle", count: due.data.length - 5 }}>
            {due.data.slice(0, 5).map((v) => (
              <QueueItem
                key={v.id}
                to={`/e/Vehicle/${v.id}`}
                title={isolate(String(v.registration ?? ""))}
                meta={isolate([v.next_service_on ? tr("By {value}", { value: formatDate(v.next_service_on) }) : "", v.odometer_km != null ? tr("{value} km", { value: formatDecimal(v.odometer_km, 0) }) : ""].filter(Boolean).join(" · "))}
              />
            ))}
          </Queue>
        </QueueGrid>
      )}
    </>
  );
}
