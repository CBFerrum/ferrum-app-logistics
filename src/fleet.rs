//! The fleet's upkeep and running costs. A fuel log's litres are more than none and its cost none
//! or more; its odometer reading is never below one logged on that day or before, nor above one
//! logged after, and it moves the vehicle's reading on, never back. A vehicle's service is due by
//! the day (its next service on or before the workspace's day) or by the kilometres (its reading
//! at its last service and every so many km since); "It Was Serviced" records the reading and the
//! next service's day. Upkeep marks the services falling due by the day. Fleet Costs gives each
//! vehicle's completed trips, their distance and cost, and its fuel over a range of days.

use crate::obj;
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use ferrum_domain::reports::{Params, ReportSource, col, read_tx};
use ferrum_domain::util::{d, elevated, id, money};
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Kernel, Result};
use ferrum_meta::effective::Schema;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct VehicleHook;
pub struct FuelHook;
pub struct FleetReports;

/// Whether a vehicle's service is due: by the day, or by the kilometres since the last one.
pub(crate) fn due(odometer: Option<i64>, last: Option<i64>, every: Option<i64>, next_on: Option<NaiveDate>, today: NaiveDate) -> bool {
    let by_day = next_on.is_some_and(|on| on <= today);
    let by_km = matches!((odometer, every), (Some(km), Some(n)) if n > 0 && km >= last.unwrap_or(0) + n);
    by_day || by_km
}

/// A whole number, as a record or a form gives it.
fn int(v: Option<&Value>) -> Option<i64> {
    v.and_then(|v| v.as_i64().or_else(|| v.as_str()?.trim().parse().ok()))
}

/// A day, when there is one.
fn day(v: Option<&Value>) -> Option<NaiveDate> {
    v.and_then(Value::as_str).and_then(|v| NaiveDate::parse_from_str(v.get(..10).unwrap_or(v), "%Y-%m-%d").ok())
}

#[async_trait]
impl DocHook for VehicleHook {
    fn name(&self) -> &'static str {
        "logistics.vehicle"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["service_due"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        let km = |k: &str| int(doc.get(k));
        let due = due(km("odometer_km"), km("last_service_km"), km("service_every_km"), day(doc.get("next_service_on")), hc.ctx.today());
        doc.insert("service_due".into(), json!(due));
        Ok(())
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        for k in ["odometer_km", "last_service_km", "service_every_km"] {
            if int(doc.get(k)).is_some_and(|v| v < 0) {
                return Err(Error::field(k, "is zero or more"));
            }
        }
        if d(doc, "cost_per_km") < Decimal::ZERO {
            return Err(Error::field("cost_per_km", "is zero or more"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        if name != "serviced" {
            return Ok(None);
        }
        let vehicle = id(doc, "id").ok_or_else(|| Error::internal("a vehicle without an id"))?;
        let km = int(params.get("odometer_km")).ok_or_else(|| Error::field("odometer_km", "is required"))?;
        let reading = int(doc.get("odometer_km")).unwrap_or(0);
        if km < reading {
            return Err(Error::field("odometer_km", format!("the odometer reads {reading} km already")));
        }
        let next = day(params.get("next_service_on"));
        if next.is_some_and(|n| n <= hc.ctx.today()) {
            return Err(Error::field("next_service_on", "the next service is after today"));
        }
        let saved = hc
            .kernel
            .update_tx(&elevated(hc.ctx), hc.tx, "Vehicle", vehicle, obj(json!({"odometer_km": km, "last_service_km": km, "next_service_on": next})), None)
            .await?;
        Ok(Some(json!({"service_due": saved.get("service_due")})))
    }
}

#[async_trait]
impl DocHook for FuelHook {
    fn name(&self) -> &'static str {
        "logistics.fuel"
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if d(doc, "litres") <= Decimal::ZERO {
            return Err(Error::field("litres", "is more than none"));
        }
        if d(doc, "cost") < Decimal::ZERO {
            return Err(Error::field("cost", "is zero or more"));
        }
        if int(doc.get("odometer_km")).is_some_and(|v| v < 0) {
            return Err(Error::field("odometer_km", "is zero or more"));
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, _old: Option<&Doc>, doc: &Doc) -> Result<()> {
        let (Some(vehicle), Some(km), Some(on)) = (id(doc, "vehicle"), int(doc.get("odometer_km")), day(doc.get("date"))) else { return Ok(()) };
        // The odometer never goes back: not below a reading on that day or before, nor above a later one.
        let (before, after): (Option<i64>, Option<i64>) = sqlx::query_as(
            "select max(odometer_km) filter (where \"date\" <= $3), min(odometer_km) filter (where \"date\" > $3) \
             from e_fuel_log where tenant_id = $1 and vehicle = $2 and id <> $4",
        )
        .bind(hc.ctx.tenant)
        .bind(vehicle)
        .bind(on)
        .bind(id(doc, "id").unwrap_or_else(Uuid::nil))
        .fetch_one(&mut **hc.tx)
        .await?;
        if let Some(b) = before.filter(|b| km < *b) {
            return Err(Error::field("odometer_km", format!("the odometer read {b} km by that day already: it never goes back")));
        }
        if let Some(a) = after.filter(|a| km > *a) {
            return Err(Error::field("odometer_km", format!("a later fuel log reads {a} km: the odometer never goes back")));
        }
        Ok(())
    }

    async fn after_save(&self, hc: &mut HookCtx<'_>, doc: &Doc, _old: Option<&Doc>) -> Result<()> {
        let (Some(vehicle), Some(km)) = (id(doc, "vehicle"), int(doc.get("odometer_km"))) else { return Ok(()) };
        let reading: Option<i64> = sqlx::query_scalar("select odometer_km from e_vehicle where tenant_id = $1 and id = $2")
            .bind(hc.ctx.tenant)
            .bind(vehicle)
            .fetch_optional(&mut **hc.tx)
            .await?
            .flatten();
        // Moved on, through the vehicle's own save, so its service falls due by the kilometres.
        if reading.is_none_or(|r| km > r) {
            hc.kernel.update_tx(&elevated(hc.ctx), hc.tx, "Vehicle", vehicle, obj(json!({"odometer_km": km})), None).await?;
        }
        Ok(())
    }
}

/// Marks the services falling due by `today` in one workspace.
pub(crate) async fn upkeep(k: &Kernel, tenant: Uuid, schema: Arc<Schema>, today: NaiveDate) -> Result<()> {
    let ctx = Ctx::system(tenant, schema);
    let mut tx = k.db.tenant_tx(tenant).await?;
    let falling: Vec<Uuid> =
        sqlx::query_scalar("select id from e_vehicle where tenant_id = $1 and next_service_on <= $2 and not coalesce(service_due, false) order by id")
            .bind(tenant)
            .bind(today)
            .fetch_all(&mut *tx)
            .await?;
    for v in falling {
        k.set_values_tx(&ctx, &mut tx, "Vehicle", v, obj(json!({"service_due": true}))).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[async_trait]
impl ReportSource for FleetReports {
    fn catalog(&self) -> Vec<Value> {
        vec![json!({"name": "logistics_fleet_costs", "label": "Fleet Costs", "module": "logistics", "entity": "Vehicle", "scoped": false,
                    "params": [{"name": "from_date", "label": "From Date", "type": "date"}, {"name": "to_date", "label": "To Date", "type": "date"},
                               {"name": "vehicle", "label": "Vehicle", "type": "link", "to": "Vehicle"}]})]
    }

    async fn run(&self, k: &Kernel, ctx: &Ctx, name: &str, params: &HashMap<String, String>) -> Result<Value> {
        if name != "logistics_fleet_costs" {
            return Err(Error::NotFound(format!("report {name}")));
        }
        let p = Params { raw: params };
        let to = p.date("to_date")?.unwrap_or_else(|| ctx.today());
        let from = p.date("from_date")?.unwrap_or_else(|| to.with_day(1).unwrap_or(to));
        if from > to {
            return Err(Error::field("from_date", "must not be after the to date"));
        }
        let vehicle = p.uuid("vehicle")?;
        let mut tx = read_tx(k, ctx).await?;
        let found: Vec<(String, i64, Decimal, Decimal, Decimal, Decimal, Option<i64>)> = sqlx::query_as(
            "select v.registration, t.trips, t.km, t.cost, f.litres, f.cost, f.driven from e_vehicle v \
             cross join lateral (select count(*) trips, coalesce(sum(distance_km), 0) km, coalesce(sum(cost), 0) cost from e_trip \
                 where tenant_id = v.tenant_id and vehicle = v.id and status = 'Completed' and \"date\" between $2 and $3) t \
             cross join lateral (select coalesce(sum(litres), 0) litres, coalesce(sum(cost), 0) cost, max(odometer_km) - min(odometer_km) driven \
                 from e_fuel_log where tenant_id = v.tenant_id and vehicle = v.id and \"date\" between $2 and $3) f \
             where v.tenant_id = $1 and ($4::uuid is null or v.id = $4) order by v.registration",
        )
        .bind(ctx.tenant)
        .bind(from)
        .bind(to)
        .bind(vehicle)
        .fetch_all(&mut *tx)
        .await?;
        tx.rollback().await?;
        let rows: Vec<Value> = found
            .into_iter()
            .map(|(registration, trips, km, trip_cost, litres, fuel_cost, driven)| {
                // Litres per 100 km by the odometer, once two readings span the range.
                let per_100 = driven.filter(|n| *n > 0).map(|n| money((litres * Decimal::from(100) / Decimal::from(n)).round_dp(2)));
                json!({"vehicle": registration, "trips": trips, "distance_km": money(km), "trip_cost": money(trip_cost), "litres": money(litres),
                       "fuel_cost": money(fuel_cost), "driven_km": driven, "litres_per_100km": per_100})
            })
            .collect();
        Ok(json!({
            "columns": [col("vehicle", "Vehicle", "data"), col("trips", "Completed Trips", "int"), col("distance_km", "Distance (km, straight line)", "decimal"),
                col("trip_cost", "Trip Cost", "currency"), col("litres", "Litres", "decimal"), col("fuel_cost", "Fuel Cost", "currency"),
                col("driven_km", "Driven (km, odometer)", "int"), col("litres_per_100km", "Litres per 100 km", "decimal")],
            "rows": rows,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::due;
    use chrono::NaiveDate;

    #[test]
    fn a_service_falls_due_by_the_day_or_the_kilometres() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        let (yesterday, tomorrow) = (today.pred_opt(), today.succ_opt());
        assert!(!due(Some(14_999), Some(5_000), Some(10_000), tomorrow, today));
        assert!(due(Some(15_000), Some(5_000), Some(10_000), tomorrow, today));
        assert!(due(Some(100), None, None, Some(today), today) && due(Some(100), None, None, yesterday, today));
        assert!(!due(Some(9_999), None, Some(10_000), None, today) && due(Some(10_000), None, Some(10_000), None, today));
        assert!(!due(None, None, Some(10_000), None, today) && !due(Some(50_000), Some(0), Some(0), None, today));
    }
}
