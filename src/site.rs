//! Logistics' endpoint for drivers (`/api/app/logistics/drive`): the dispatched trips of a day that
//! the signed-in person drives, with their stops, found by the address their driver's record
//! gives. A driver reads no other driver's record, nor anyone else's trips, this way.

use chrono::NaiveDate;
use ferrum_domain::util::{id, sys};
use ferrum_kernel::{Error, Filter, Kernel, Result, Site, SiteRequest, SiteResponse};
use serde_json::{Value, json};

pub struct LogisticsSite;

#[async_trait::async_trait]
impl Site for LogisticsSite {
    async fn handle(&self, k: &Kernel, req: SiteRequest) -> Result<SiteResponse> {
        let user = req.user.as_ref().ok_or_else(|| Error::Unauthorized("sign in first".into()))?;
        if (req.method.as_str(), req.path.trim_matches('/')) != ("GET", "drive") {
            return Err(Error::NotFound(format!("logistics endpoint {} {}", req.method, req.path)));
        }
        let day = match req.query.get("day").and_then(Value::as_str) {
            Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|_| Error::field("day", "is YYYY-MM-DD"))?,
            None => chrono::Utc::now().date_naive(),
        };
        let email = user.email.to_lowercase();
        let mut trips = vec![];
        if !email.is_empty() {
            let sctx = sys(user);
            let mut tx = k.db.tenant_tx(user.tenant).await?;
            let drivers = k.find_tx(&sctx, &mut tx, "Driver", &[Filter::eq("email", email)], None, 5).await?;
            for driver in drivers.iter().filter_map(|d| id(d, "id")) {
                let found = k
                    .find_tx(
                        &sctx,
                        &mut tx,
                        "Trip",
                        &[Filter::eq("driver", driver.to_string()), Filter::eq("date", day.to_string()), Filter::eq("status", "Dispatched")],
                        Some("number asc"),
                        20,
                    )
                    .await?;
                for t in found.iter().filter_map(|t| id(t, "id")) {
                    trips.push(Value::Object(k.get_tx(&sctx, &mut tx, "Trip", t, false).await?));
                }
            }
            tx.commit().await?;
        }
        let mut r = SiteResponse::json(200, &json!({"day": day.to_string(), "trips": trips}));
        r.headers.push(("cache-control".into(), "no-store".into()));
        Ok(r)
    }
}
