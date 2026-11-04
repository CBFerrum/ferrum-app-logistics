//! Proof of delivery. Only the trip's driver, or a dispatcher, records what happened at a stop of a
//! dispatched trip, once for each stop. Delivered, it names who took the goods and carries their
//! signature or a photo; any other outcome fails the stop. The proof is kept as it was recorded.

use crate::status;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::util::{elevated, id, rows, s, sys};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result};
use serde_json::{Value, json};

pub struct ProofHook;

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
        if s(doc, "outcome") == Some("Delivered") {
            if s(doc, "received_by").is_none() {
                return Err(Error::field("received_by", "a delivery names who took the goods"));
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
        let dispatcher = hc.ctx.is_admin || hc.ctx.roles.iter().any(|r| r == "Dispatcher");
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
        Ok(())
    }

    async fn after_save(&self, hc: &mut HookCtx<'_>, doc: &Doc, old: Option<&Doc>) -> Result<()> {
        if old.is_some() {
            return Ok(());
        }
        let (Some(trip_id), Some(n)) = (id(doc, "trip"), doc.get("stop").and_then(Value::as_i64)) else { return Ok(()) };
        let system = elevated(hc.ctx);
        let trip = hc.kernel.get_tx(&system, hc.tx, "Trip", trip_id, true).await?;
        let outcome = if s(doc, "outcome") == Some("Delivered") { "Delivered" } else { "Failed" };
        let stops: Vec<Value> = rows(&trip, "stops")
            .into_iter()
            .map(|mut r| {
                if r.get("sequence").and_then(Value::as_i64) == Some(n) {
                    r.insert("status".into(), json!(outcome));
                    r.insert("delivered_at".into(), json!(Utc::now().to_rfc3339()));
                }
                Value::Object(r)
            })
            .collect();
        hc.kernel.update_tx(&system, hc.tx, "Trip", trip_id, crate::obj(json!({"stops": stops})), None).await?;
        Ok(())
    }
}
