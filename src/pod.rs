//! Proof of delivery. Only the trip's driver, or a dispatcher, records what happened at a stop of a
//! dispatched trip, once for each stop. Delivered (a delivery) or collected (a collection), it names
//! who took or handed over the goods and carries their signature or a photo; any other outcome
//! fails the stop. A collection's return note is then submitted, so its goods are back in stock.
//! A proof says when it happened, by the driver's phone, so one recorded offline and sent later
//! keeps its time: not later than now (a few minutes' grace for the phone's clock), nor before the
//! trip went out. A stop that happened after its window closed, on the workspace's clock, is
//! marked late. The proof is kept as it was recorded.

use crate::status;
use async_trait::async_trait;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use ferrum_domain::util::{elevated, id, rows, s, sys};
use ferrum_kernel::value::parse_datetime;
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Result};
use serde_json::{Value, json};

pub struct ProofHook;

/// How far ahead of the server a phone's clock may run.
const GRACE: Duration = Duration::minutes(5);

/// Whether a stop that happened at `time` on `on` is late: after its window closed on the trip's
/// `day`, or on a later day. Days are `YYYY-MM-DD` and times `HH:MM`, so they compare as text.
pub(crate) fn late(closes: Option<&str>, day: &str, on: &str, time: &str) -> bool {
    closes.is_some_and(|w| on > day || (on == day && time > w))
}

/// When a proof says it happened.
fn happened(doc: &Doc) -> Result<DateTime<Utc>> {
    s(doc, "happened_at").map_or_else(|| Ok(Utc::now()), |v| parse_datetime(v).map_err(|m| Error::field("happened_at", m)))
}

/// The workspace's time zone, UTC when it names none this build knows.
fn zone(ctx: &Ctx) -> chrono_tz::Tz {
    ctx.schema.timezone.parse().unwrap_or(chrono_tz::Tz::UTC)
}

/// Whether `email` drives the trip, as its driver's record says.
async fn drives(hc: &mut HookCtx<'_>, trip: &Doc) -> Result<bool> {
    let Some(driver) = id(trip, "driver") else { return Ok(false) };
    let email: Option<String> = sqlx::query_scalar("select email from e_driver where tenant_id = $1 and id = $2")
        .bind(hc.ctx.tenant)
        .bind(driver)
        .fetch_optional(&mut **hc.tx)
        .await?
        .flatten();
    Ok(!hc.ctx.email.is_empty() && email.is_some_and(|e| e.eq_ignore_ascii_case(&hc.ctx.email)))
}

#[async_trait]
impl DocHook for ProofHook {
    fn name(&self) -> &'static str {
        "logistics.proof"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["recorded_by", "happened_at"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if doc.get("recorded_by").is_none_or(Value::is_null) {
            doc.insert("recorded_by".into(), json!(hc.ctx.email));
        }
        if doc.get("happened_at").is_none_or(Value::is_null) {
            doc.insert("happened_at".into(), json!(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)));
        }
        Ok(())
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if matches!(s(doc, "outcome"), Some("Delivered" | "Collected")) {
            if s(doc, "received_by").is_none() {
                return Err(Error::field("received_by", "a delivery names who took the goods, a collection who handed them over"));
            }
            if s(doc, "signature").is_none() && s(doc, "photo").is_none() {
                return Err(Error::field("signature", "a delivery carries a signature or a photo"));
            }
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        if old.is_some() {
            return if hc.ctx.is_system { Ok(()) } else { Err(Error::invalid("a proof of delivery is kept as it was recorded")) };
        }
        let trip_id = id(doc, "trip").ok_or_else(|| Error::field("trip", "is required"))?;
        let trip = hc.kernel.get_tx(&sys(hc.ctx), hc.tx, "Trip", trip_id, true).await?;
        if status(&trip, "Planned") != "Dispatched" {
            return Err(Error::field("trip", "is not out: dispatch it first"));
        }
        let at = happened(doc)?;
        if at > Utc::now() + GRACE {
            return Err(Error::field("happened_at", "is later than now"));
        }
        if s(&trip, "dispatched_at").and_then(|v| parse_datetime(v).ok()).is_some_and(|out| at + GRACE < out) {
            return Err(Error::field("happened_at", "is before the trip went out"));
        }
        let dispatcher = hc.ctx.is_admin || hc.ctx.roles.iter().any(|r| r == "Fleet Dispatcher");
        if !dispatcher && !drives(hc, &trip).await? {
            return Err(Error::Forbidden("only the trip's driver records its deliveries".into()));
        }
        let n = doc.get("stop").and_then(Value::as_i64).ok_or_else(|| Error::field("stop", "is required"))?;
        let stop = rows(&trip, "stops")
            .into_iter()
            .find(|r| r.get("sequence").and_then(Value::as_i64) == Some(n))
            .ok_or_else(|| Error::field("stop", format!("the trip has no stop {n}")))?;
        if status(&stop, "Pending") != "Pending" {
            return Err(Error::field("stop", format!("stop {n} is {} already", status(&stop, "Pending").to_lowercase())));
        }
        let collection = s(&stop, "kind") == Some("Collection");
        match s(doc, "outcome") {
            Some("Collected") if !collection => return Err(Error::field("outcome", format!("stop {n} is a delivery: it is delivered, not collected"))),
            Some("Delivered") if collection => return Err(Error::field("outcome", format!("stop {n} is a collection: it is collected, not delivered"))),
            _ => {}
        }
        Ok(())
    }

    async fn after_save(&self, hc: &mut HookCtx<'_>, doc: &Doc, old: Option<&Doc>) -> Result<()> {
        if old.is_some() {
            return Ok(());
        }
        let (Some(trip_id), Some(n)) = (id(doc, "trip"), doc.get("stop").and_then(Value::as_i64)) else { return Ok(()) };
        let system = elevated(hc.ctx);
        let trip = hc.kernel.get_tx(&system, hc.tx, "Trip", trip_id, true).await?;
        let outcome = match s(doc, "outcome") {
            Some("Delivered") => "Delivered",
            Some("Collected") => "Collected",
            _ => "Failed",
        };
        // Late, on the workspace's clock when it happened: after its window closed on the trip's
        // day, or a later day.
        let at = happened(doc)?;
        let local = at.with_timezone(&zone(hc.ctx));
        let (on, time) = (local.date_naive().to_string(), local.format("%H:%M").to_string());
        let day = s(&trip, "date").unwrap_or_default().to_string();
        let mut back = None;
        let stops: Vec<Value> = rows(&trip, "stops")
            .into_iter()
            .map(|mut r| {
                if r.get("sequence").and_then(Value::as_i64) == Some(n) {
                    let late = late(s(&r, "window_to").and_then(crate::trips::clock).as_deref(), &day, &on, &time);
                    if outcome == "Collected" {
                        back = id(&r, "return_note");
                    }
                    r.insert("status".into(), json!(outcome));
                    r.insert("delivered_at".into(), json!(at.to_rfc3339_opts(SecondsFormat::Millis, true)));
                    r.insert("late".into(), json!(late));
                }
                Value::Object(r)
            })
            .collect();
        hc.kernel.update_tx(&system, hc.tx, "Trip", trip_id, crate::obj(json!({"stops": stops})), None).await?;
        // The goods collected are back in stock: their return note submitted.
        if let Some(note) = back {
            hc.kernel.submit_tx(&system, hc.tx, "DeliveryNote", note, None).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::late;

    #[test]
    fn late_once_the_window_has_closed() {
        let day = "2026-11-10";
        assert!(!late(Some("10:00"), day, day, "09:59") && !late(Some("10:00"), day, day, "10:00"));
        assert!(late(Some("10:00"), day, day, "10:01") && late(Some("10:00"), day, "2026-11-11", "08:00"));
        assert!(!late(Some("10:00"), day, "2026-11-09", "23:00") && !late(None, day, "2026-12-01", "23:00"));
    }
}
