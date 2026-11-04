//! Logistics and fleets (FERRUM-INDUSTRY-009): depots, vehicles and drivers, trips dispatched
//! with their stops in order, and proof of delivery.
//!
//! * `geo`: distances as the crow flies, and stops ordered nearest first from the depot. Road
//!   routing, geocoding and live tracking need a maps provider (BLOCK-MAPS-KEY).
//! * `trips`: a trip is dispatched only with a vehicle on the road that carries its load, a driver
//!   licensed on the day, neither out on another trip that day, and its delivery notes submitted
//!   and on no other trip.
//! * `pod`: the trip's driver records each stop delivered or failed, with who took the goods and
//!   their signature or a photo.
//! * `site`: a driver's dispatched trips of the day, found by the address they signed in with.
//!
//! This app's own design; no code was copied (docs/PROVENANCE.md). Hooks are registered per entity,
//! so a workspace without the `logistics` package never runs them.

mod geo;
mod pod;
mod site;
mod trips;

use ferrum_domain::reports::ReportSource;
use ferrum_domain::util::s;
use ferrum_kernel::{Doc, HookRegistry, Kernel, Result};
use serde_json::Value;
use std::sync::Arc;

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
    hooks.register("Trip", Arc::new(trips::TripHook)).register("ProofOfDelivery", Arc::new(pod::ProofHook));
    hooks.add_site("logistics", Arc::new(site::LogisticsSite));
}

/// This app's reports: none of its own yet.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![]
}

/// Upkeep: nothing runs by itself.
pub async fn maintenance(_k: &Kernel) -> Result<()> {
    Ok(())
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
