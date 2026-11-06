//! Logistics and fleets (FERRUM-INDUSTRY-009): depots, vehicles and drivers, trips dispatched
//! with their stops in order, deliveries and collections in their windows, proof of delivery, and
//! the fleet's fuel, services and costs.
//!
//! * `geo`: distances as the crow flies, and stops ordered nearest first from the depot. Road
//!   routing, geocoding and live tracking need a maps provider (BLOCK-MAPS-KEY).
//! * `trips`: stops ordered by their windows, nearest first among those opening together; a trip
//!   is dispatched only with a vehicle on the road that carries its load, a driver licensed on the
//!   day, neither out on another trip at the time, its delivery notes submitted and its return
//!   notes in draft, each on no other open trip; completed, its cost is its distance at the
//!   vehicle's rate.
//! * `pod`: the trip's driver records each stop delivered, collected or failed, with who took or
//!   handed over the goods and their signature or a photo; a collection's return note is then
//!   submitted, and a stop proved after its window closed is late.
//! * `fleet`: fuel logs moving the odometer on, services due by the day or the kilometres, and
//!   Fleet Costs.
//! * `site`: a driver's dispatched trips of the day, found by the address they signed in with.
//!
//! This app's own design; no code was copied (docs/PROVENANCE.md). Hooks are registered per entity,
//! so a workspace without the `logistics` package never runs them.

mod fleet;
mod geo;
mod pod;
mod site;
mod trips;

use chrono::NaiveDate;
use ferrum_domain::reports::ReportSource;
use ferrum_domain::util::s;
use ferrum_kernel::{Ctx, Doc, Error, HookRegistry, Kernel, Result};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

include!(concat!(env!("OUT_DIR"), "/packages.rs"));
include!(concat!(env!("OUT_DIR"), "/ui.rs"));

/// This app's manifest (`app.yaml`).
pub const MANIFEST: &str = include_str!("../app.yaml");

/// This app's metadata packages, embedded at build time.
pub fn packages() -> std::result::Result<Vec<ferrum_meta::Package>, ferrum_meta::MetaError> {
    ferrum_meta::catalog::load_embedded(EMBEDDED)
}

/// Registers this app's document hooks, and drivers' endpoint.
pub fn register(hooks: &mut HookRegistry) {
    hooks
        .register("Trip", Arc::new(trips::TripHook))
        .register("ProofOfDelivery", Arc::new(pod::ProofHook))
        .register("Vehicle", Arc::new(fleet::VehicleHook))
        .register("FuelLog", Arc::new(fleet::FuelHook));
    hooks.add_site("logistics", Arc::new(site::LogisticsSite));
}

/// This app's reports: Fleet Costs.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![Arc::new(fleet::FleetReports)]
}

/// Upkeep: in each workspace with Logistics, services falling due on its own day are marked due.
pub async fn maintenance(k: &Kernel) -> Result<()> {
    upkeep_all(k, None).await
}

/// Upkeep as on `today` in every workspace, for tests.
pub async fn maintenance_on(k: &Kernel, today: NaiveDate) -> Result<()> {
    upkeep_all(k, Some(today)).await
}

async fn upkeep_all(k: &Kernel, day: Option<NaiveDate>) -> Result<()> {
    let mut tx = k.db.system_tx().await?;
    let tenants: Vec<Uuid> = sqlx::query_scalar("select id from tenant where status = 'active' order by id").fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let mut first_error: Option<Error> = None;
    for tenant in tenants {
        let done = match k.registry.schema(tenant).await {
            Ok(s) if s.packages.iter().any(|p| p == "logistics") => {
                let today = day.unwrap_or_else(|| Ctx::system(tenant, s.clone()).today());
                fleet::upkeep(k, tenant, s, today).await
            }
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        };
        if let Err(e) = done {
            first_error.get_or_insert(e);
        }
    }
    first_error.map_or(Ok(()), Err)
}

/// A document from JSON.
pub(crate) fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap_or_default()
}

/// A status, for messages.
pub(crate) fn status<'a>(doc: &'a Doc, default: &'a str) -> &'a str {
    s(doc, "status").unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_is_a_release_the_platform_takes() {
        let m = ferrum_apps::AppManifest::parse(MANIFEST).expect("app.yaml");
        assert_eq!(m.routes(), vec![("logistics".to_string(), vec![])]);
        let files = UI.iter().map(|(p, b)| (p.to_string(), b.to_vec())).collect();
        let ui = ferrum_apps::ui::prepare("logistics", files).expect("ui/dist is an interface release");
        assert!(UI.iter().any(|(p, _)| *p == "index.html") && !UI.iter().any(|(p, _)| p.starts_with('.')), "{}", ui.digest);
    }
}
