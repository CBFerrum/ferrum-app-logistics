//! Trips. A trip's stops are numbered in the order they are visited, its load is their weight, and
//! its distance is the straight line from the depot through them and back when every one has its
//! place. Ordering the stops visits the nearest first. Dispatching wants a vehicle on the road that
//! carries the load, a driver active and licensed on the day, neither out on another trip that
//! day, and each delivery note submitted and on no other open trip; the notes then carry the
//! vehicle. A dispatched trip keeps its stops; it completes once each is delivered or failed.

use crate::geo::{Point, nearest_first, round_trip};
use crate::{obj, status};
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::util::{d, elevated, id, money, rows, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Value, json};
use std::collections::HashSet;
use uuid::Uuid;

pub struct TripHook;

/// A record's place, when it has both its latitude and its longitude.
pub(crate) fn place(doc: &Doc) -> Option<Point> {
    let (la, lo) = (doc.get("latitude").filter(|v| !v.is_null())?, doc.get("longitude").filter(|v| !v.is_null())?);
    let num = |v: &Value| v.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| v.as_f64());
    Some((num(la)?, num(lo)?))
}

/// What a trip visits: each stop's customer, note, address and weight, in order.
fn route(doc: &Doc) -> Vec<(Option<String>, Option<String>, Option<String>, Decimal)> {
    rows(doc, "stops")
        .iter()
        .map(|r| (s(r, "customer").map(str::to_string), s(r, "delivery_note").map(str::to_string), s(r, "address").map(str::to_string), d(r, "weight_kg")))
        .collect()
}

#[async_trait]
impl DocHook for TripHook {
    fn name(&self) -> &'static str {
        "logistics.trip"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["stops", "total_weight", "distance_km"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        let mut stops = rows(doc, "stops");
        for (i, r) in stops.iter_mut().enumerate() {
            r.insert("sequence".into(), json!(i + 1));
        }
        let load: Decimal = stops.iter().map(|r| d(r, "weight_kg")).sum();
        let depot = match id(doc, "depot") {
            Some(dp) => sqlx::query_as::<_, (Option<Decimal>, Option<Decimal>)>("select latitude, longitude from e_depot where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(dp)
                .fetch_optional(&mut **hc.tx)
                .await?
                .and_then(|(a, b)| Some((a?.to_f64()?, b?.to_f64()?))),
            None => None,
        };
        let places: Option<Vec<Point>> = stops.iter().map(place).collect();
        let distance = match (depot, places) {
            (Some(start), Some(p)) if !p.is_empty() => json!(format!("{:.1}", round_trip(start, &p))),
            _ => Value::Null,
        };
        doc.insert("stops".into(), Value::Array(stops.into_iter().map(Value::Object).collect()));
        doc.insert("total_weight".into(), money(load));
        doc.insert("distance_km".into(), distance);
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if matches!(status(doc, "Planned"), "Completed" | "Cancelled") {
            return Ok(());
        }
        let stops = rows(doc, "stops");
        if stops.iter().any(|r| d(r, "weight_kg") < Decimal::ZERO) {
            return Err(Error::field("stops", "a stop's weight is zero or more"));
        }
        let mut notes = HashSet::new();
        for r in &stops {
            let Some(note) = id(r, "delivery_note") else { continue };
            if !notes.insert(note) {
                return Err(Error::field("stops", "a delivery note is one stop of a trip"));
            }
            let row: Option<(i16, Option<String>)> = sqlx::query_as("select docstatus, number from e_delivery_note where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(note)
                .fetch_optional(&mut **hc.tx)
                .await?;
            let (docstatus, number) = row.ok_or_else(|| Error::field("stops", "a delivery note there is not this workspace's"))?;
            let number = number.unwrap_or_default();
            if docstatus != 1 {
                return Err(Error::field("stops", format!("delivery note {number} is not submitted: the goods have not left the books")));
            }
            let other: Option<String> = sqlx::query_scalar(
                "select t.number from e_trip t join e_trip__stops st on st.tenant_id = t.tenant_id and st.parent_id = t.id \
                 where t.tenant_id = $1 and st.delivery_note = $2 and t.id <> $3 and t.status in ('Planned', 'Dispatched') limit 1",
            )
            .bind(hc.ctx.tenant)
            .bind(note)
            .bind(id(doc, "id").unwrap_or_else(Uuid::nil))
            .fetch_optional(&mut **hc.tx)
            .await?;
            if let Some(trip) = other {
                return Err(Error::field("stops", format!("delivery note {number} is on {trip} already")));
            }
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        if hc.ctx.is_system {
            return Ok(());
        }
        let Some(o) = old else { return Ok(()) };
        match status(o, "Planned") {
            "Planned" => Ok(()),
            "Dispatched"
                if route(o) == route(doc) && s(o, "vehicle") == s(doc, "vehicle") && s(o, "driver") == s(doc, "driver") && s(o, "date") == s(doc, "date") =>
            {
                Ok(())
            }
            "Dispatched" => Err(Error::invalid("a dispatched trip keeps its vehicle, driver and stops: cancel it and plan another")),
            st => Err(Error::invalid(format!("a {} trip is not changed", st.to_lowercase()))),
        }
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        let trip = id(doc, "id").ok_or_else(|| Error::internal("a trip without an id"))?;
        let system = elevated(hc.ctx);
        let now = Utc::now().to_rfc3339();
        match name {
            "plan_route" => {
                let depot = id(doc, "depot").ok_or_else(|| Error::field("depot", "is required"))?;
                let depot = hc.kernel.get_tx(&system, hc.tx, "Depot", depot, false).await?;
                let start = place(&depot).ok_or_else(|| Error::field("depot", "needs its latitude and longitude to order the stops"))?;
                let stops = rows(doc, "stops");
                let places: Vec<Point> = stops
                    .iter()
                    .map(place)
                    .collect::<Option<_>>()
                    .ok_or_else(|| Error::field("stops", "each stop needs its latitude and longitude to order them"))?;
                let order = nearest_first(start, &places);
                let reordered: Vec<Value> = order.into_iter().map(|i| Value::Object(stops[i].clone())).collect();
                let saved = hc.kernel.update_tx(&system, hc.tx, "Trip", trip, obj(json!({"stops": reordered})), None).await?;
                Ok(Some(json!({"distance_km": saved.get("distance_km"), "stops": rows(&saved, "stops").iter().map(|r| s(r, "address")).collect::<Vec<_>>()})))
            }
            "dispatch" => {
                dispatchable(hc, doc).await?;
                hc.kernel.set_values_tx(&system, hc.tx, "Trip", trip, obj(json!({"status": "Dispatched", "dispatched_at": now}))).await?;
                // The notes carry the vehicle they went out on.
                let registration: Option<String> = sqlx::query_scalar("select registration from e_vehicle where tenant_id = $1 and id = $2")
                    .bind(hc.ctx.tenant)
                    .bind(id(doc, "vehicle"))
                    .fetch_optional(&mut **hc.tx)
                    .await?;
                for note in rows(doc, "stops").iter().filter_map(|r| id(r, "delivery_note")) {
                    hc.kernel.set_values_tx(&system, hc.tx, "DeliveryNote", note, obj(json!({"vehicle_no": registration, "transporter": "Own fleet"}))).await?;
                }
                Ok(Some(json!({"status": "Dispatched"})))
            }
            "complete" => {
                let open: Vec<i64> =
                    rows(doc, "stops").iter().filter(|r| status(r, "Pending") == "Pending").filter_map(|r| r.get("sequence").and_then(Value::as_i64)).collect();
                if !open.is_empty() {
                    return Err(Error::invalid(format!("stops {open:?} have no proof of delivery yet")));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "Trip", trip, obj(json!({"status": "Completed", "completed_at": now}))).await?;
                Ok(Some(json!({"status": "Completed"})))
            }
            "cancel_trip" => {
                if rows(doc, "stops").iter().any(|r| status(r, "Pending") == "Delivered") {
                    return Err(Error::invalid("some of this trip is delivered: complete it, failing the rest"));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "Trip", trip, obj(json!({"status": "Cancelled"}))).await?;
                Ok(Some(json!({"status": "Cancelled"})))
            }
            _ => Ok(None),
        }
    }
}

/// Refused unless the trip has stops, its vehicle is on the road and carries the load, its driver
/// is active and licensed on the day, and neither is out on another trip that day.
async fn dispatchable(hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
    let tenant = hc.ctx.tenant;
    if rows(doc, "stops").is_empty() {
        return Err(Error::field("stops", "a trip goes somewhere: add its stops"));
    }
    let day = s(doc, "date").ok_or_else(|| Error::field("date", "is required"))?.to_string();
    let (vehicle, driver) = (id(doc, "vehicle"), id(doc, "driver"));
    let v: (String, Option<String>, Option<Decimal>) =
        sqlx::query_as("select registration, status, capacity_kg from e_vehicle where tenant_id = $1 and id = $2")
            .bind(tenant)
            .bind(vehicle)
            .fetch_optional(&mut **hc.tx)
            .await?
            .ok_or_else(|| Error::field("vehicle", "is required"))?;
    if v.1.as_deref() == Some("Off Road") {
        return Err(Error::field("vehicle", format!("{} is off the road", v.0)));
    }
    let load = d(doc, "total_weight");
    if load > v.2.unwrap_or_default() {
        return Err(Error::field("vehicle", format!("{} carries {} kg, and the load is {load} kg", v.0, v.2.unwrap_or_default())));
    }
    let dr: (String, Option<String>, Option<chrono::NaiveDate>) =
        sqlx::query_as("select full_name, status, licence_expires_on from e_driver where tenant_id = $1 and id = $2")
            .bind(tenant)
            .bind(driver)
            .fetch_optional(&mut **hc.tx)
            .await?
            .ok_or_else(|| Error::field("driver", "is required"))?;
    if dr.1.as_deref() != Some("Active") {
        return Err(Error::field("driver", format!("{} is not active", dr.0)));
    }
    let on = chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d").map_err(|_| Error::field("date", "is a date"))?;
    if dr.2.is_none_or(|e| e < on) {
        return Err(Error::field("driver", format!("{}'s licence is not valid on {day}", dr.0)));
    }
    let busy: Option<String> = sqlx::query_scalar(
        "select number from e_trip where tenant_id = $1 and \"date\" = $2 and status = 'Dispatched' and id <> $5 and (vehicle = $3 or driver = $4) limit 1",
    )
    .bind(tenant)
    .bind(on)
    .bind(vehicle)
    .bind(driver)
    .bind(id(doc, "id"))
    .fetch_optional(&mut **hc.tx)
    .await?;
    if let Some(other) = busy {
        return Err(Error::invalid(format!("the vehicle or the driver is out on {other} that day")));
    }
    Ok(())
}
