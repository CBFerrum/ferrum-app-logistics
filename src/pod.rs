//! Proof of delivery. Only the trip's driver, or a dispatcher, records what happened at a stop of a
//! dispatched trip, once for each stop. Delivered (a delivery) or collected (a collection), it names
//! who took or handed over the goods and carries their signature or a photo; any other outcome
//! fails the stop. A collection's return note is then submitted, so its goods are back in stock. A
//! stop recorded after its window closed, on the workspace's clock, is marked late. The proof is
//! kept as it was recorded.

use crate::status;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::util::{elevated, id, rows, s, sys};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result};
use serde_json::{Value, json};

pub struct ProofHook;

/// Whether a stop proved at `time` on `today` is late: after its window closed on the trip's
/// `day`, or on a later day. Days are `YYYY-MM-DD` and times `HH:MM`, so they compare as text.
pub(crate) fn late(closes: Option<&str>, day: &str, today: &str, time: &str) -> bool {
    closes.is_some_and(|w| today > day || (today == day && time > w))
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
        &["recorded_by"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if doc.get("recorded_by").is_none_or(Value::is_null) {
            doc.insert("recorded_by".into(), json!(hc.ctx.email));
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
        // Late, on the workspace's clock: after its window closed on the trip's day, or a later day.
        let now = hc.ctx.now_local();
        let (today, time) = (now.date_naive().to_string(), now.format("%H:%M").to_string());
        let day = s(&trip, "date").unwrap_or_default().to_string();
        let mut back = None;
        let stops: Vec<Value> = rows(&trip, "stops")
            .into_iter()
            .map(|mut r| {
                if r.get("sequence").and_then(Value::as_i64) == Some(n) {
                    let late = late(s(&r, "window_to").and_then(crate::trips::clock).as_deref(), &day, &today, &time);
                    if outcome == "Collected" {
                        back = id(&r, "return_note");
                    }
                    r.insert("status".into(), json!(outcome));
                    r.insert("delivered_at".into(), json!(Utc::now().to_rfc3339()));
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
