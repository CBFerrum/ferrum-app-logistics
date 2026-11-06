//! A delivery day (FERRUM-INDUSTRY-009) against a real PostgreSQL
//! (`FERRUM_LOGISTICS_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_logistics_test`;
//! reset first): a trip planned from submitted delivery notes, its stops ordered nearest first from
//! the depot; dispatch refused for a vehicle off the road or too small, a driver unlicensed or
//! inactive, and a vehicle already out; the notes carrying the vehicle; proofs of delivery recorded
//! only by the trip's driver, once a stop, with who took the goods and their signature or a photo;
//! completing, at the vehicle's rate, and cancelling; a driver's own stops; a vehicle out again the
//! same day; collections with their return notes, stops ordered by their windows and proved late;
//! fuel logs that never wind the odometer back, services due by the kilometres and by the day;
//! Fleet Costs; and who may do what.

use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

fn dec(v: &Value) -> f64 {
    v.as_str().map(|s| s.parse().unwrap()).or_else(|| v.as_f64()).unwrap_or_else(|| panic!("not a number: {v}"))
}

struct T {
    k: Arc<Kernel>,
    tenant: Uuid,
    admin: Ctx,
}

impl T {
    async fn insert(&self, ctx: &Ctx, entity: &str, v: Value) -> Doc {
        self.k.insert(ctx, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))
    }
    async fn get(&self, entity: &str, id: Uuid) -> Doc {
        self.k.get(&self.admin, entity, id, false).await.unwrap_or_else(|e| panic!("get {entity}: {e:?}"))
    }
    async fn find_one(&self, entity: &str, filters: Value) -> Doc {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(50), ..Default::default() };
        let rows = self.k.list(&self.admin, entity, &q).await.unwrap().rows;
        assert_eq!(rows.len(), 1, "{entity} {filters}");
        rows[0].clone()
    }
    async fn user(&self, email: &str, roles: &[&str]) -> Ctx {
        let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
        let id = self.k.create_user(&self.admin, email, email, Some("Correct-Horse-9"), &roles, false).await.expect("user");
        let s = self.k.login("fleet", email, "Correct-Horse-9").await.expect("login");
        self.k.load_ctx(self.tenant, id, Some(s.token_version), "it".into()).await.expect("ctx")
    }
    async fn act(&self, ctx: &Ctx, id: Uuid, action: &str) -> Result<Value, Error> {
        self.k.run_action(ctx, "Trip", id, action, Value::Null).await
    }
    async fn stops(&self, trip: Uuid) -> Vec<(i64, String, String)> {
        self.get("Trip", trip).await["stops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| (s["sequence"].as_i64().unwrap(), s["address"].as_str().unwrap().to_string(), s["status"].as_str().unwrap().to_string()))
            .collect()
    }
}

/// A call to Logistics' endpoint as `who` (`/api/app/logistics/{path}`).
async fn call(k: &Kernel, who: &Ctx, path: &str, query: Value) -> Result<Value, Error> {
    let site = k.hooks.site("logistics").expect("logistics' endpoint").clone();
    let req = ferrum_kernel::SiteRequest {
        tenant: who.tenant,
        user: Some(who.clone()),
        method: "GET".into(),
        path: format!("/{path}"),
        query: query.as_object().cloned().unwrap_or_default(),
        headers: Default::default(),
        body: vec![],
        client_ip: "203.0.113.9".into(),
    };
    let r = site.handle(k, req).await?;
    Ok(serde_json::from_slice(&r.body).unwrap())
}

fn refused<V: std::fmt::Debug>(r: Result<V, Error>, kind: &str, says: &str) {
    let msg = format!("{r:?}");
    let ok = matches!(
        (&r, kind),
        (Err(Error::Invalid(_) | Error::Validation(_)), "invalid") | (Err(Error::Forbidden(_)), "forbidden") | (Err(Error::NotFound(_)), "not found")
    );
    assert!(ok && msg.contains(says), "expected {kind} saying {says:?}, got {msg}");
}

#[test]
fn a_delivery_day() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url = std::env::var("FERRUM_LOGISTICS_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_logistics_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 16, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
    packages.extend(ferrum_app_logistics::packages().expect("app packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_erp::register(&mut hooks);
    ferrum_app_logistics::register(&mut hooks);
    for r in ferrum_app_logistics::reports() {
        ferrum_domain::reports::add_source(r);
    }
    let settings = KernelSettings { secret_key: Some(SecretKey::new([7u8; 32])), ..Default::default() };
    let k = Kernel::boot(db, catalog, hooks, settings, None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    ferrum_app_erp::maintenance(&k).await.expect("partitions");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "fleet".into(),
            name: "Fleet Foods".into(),
            admin_email: "admin@fleet.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["logistics".into()],
            settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("fleet", "admin@fleet.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let t = T { k: k.clone(), tenant, admin };
    let a = t.admin.clone();

    // ───────────── the books, the notes and the fleet ─────────────
    let inr = t.find_one("Currency", json!({"code": "INR"})).await;
    let company = uid(&t.insert(&a, "Company", json!({"company_name": "Fleet Foods", "abbr": "FF", "default_currency": inr["id"]})).await["id"]);
    let services = t.find_one("ItemGroup", json!({"group_name": "Services"})).await;
    let nos = t.find_one("Uom", json!({"uom_name": "Nos"})).await;
    let parcel = uid(&t
        .insert(&a, "Item", json!({"item_code": "HAMPER", "item_name": "Hamper", "item_group": services["id"], "stock_uom": nos["id"], "is_stock_item": false}))
        .await["id"]);
    let stores = t.find_one("Warehouse", json!({"warehouse_name": "Stores", "company": company.to_string()})).await;
    let customer = |name: &str| json!({"customer_name": name, "customer_type": "Company"});
    let mut notes = vec![];
    for name in ["Ana's Cafe", "Bo Books", "Cy Hardware", "Dee Deli"] {
        let c = uid(&t.insert(&a, "Customer", customer(name)).await["id"]);
        let dn = uid(&t
            .insert(
                &a,
                "DeliveryNote",
                json!({"company": company.to_string(), "customer": c.to_string(), "set_warehouse": stores["id"], "items": [{"item": parcel.to_string(), "qty": "1", "rate": "500"}]}),
            )
            .await["id"]);
        notes.push((c, dn));
    }
    for (_, dn) in &notes[..3] {
        t.k.submit(&a, "DeliveryNote", *dn, None).await.expect("submitted");
    }
    let dispatcher = t.user("desk@fleet.test", &["Fleet Dispatcher"]).await;
    let dan = t.user("dan@fleet.test", &["Driver"]).await;
    let eve = t.user("eve@fleet.test", &["Driver"]).await;
    let hub = uid(&t.insert(&dispatcher, "Depot", json!({"depot_name": "North Hub", "latitude": "12.9700", "longitude": "77.5900"})).await["id"]);
    let south = uid(&t.insert(&dispatcher, "Depot", json!({"depot_name": "South Yard"})).await["id"]);
    let vehicle = |reg: &str, kg: &str, status: &str| json!({"registration": reg, "capacity_kg": kg, "depot": hub.to_string(), "status": status});
    let van1 = uid(&t.insert(&dispatcher, "Vehicle", vehicle("VAN-1", "500", "Available")).await["id"]);
    let van2 = uid(&t.insert(&dispatcher, "Vehicle", vehicle("VAN-2", "500", "Off Road")).await["id"]);
    let bike = uid(&t.insert(&dispatcher, "Vehicle", vehicle("BIKE-1", "100", "Available")).await["id"]);
    t.k.update(&dispatcher, "Vehicle", van1, obj(json!({"cost_per_km": "12.50"})), None).await.expect("VAN-1's rate");
    let driver = |name: &str, email: &str, expires: &str, status: &str| json!({"full_name": name, "email": email, "licence_number": format!("DL-{name}"), "licence_expires_on": expires, "status": status});
    let dan_d = uid(&t.insert(&dispatcher, "Driver", driver("Dan", "dan@fleet.test", "2027-12-31", "Active")).await["id"]);
    let eve_d = uid(&t.insert(&dispatcher, "Driver", driver("Eve", "eve@fleet.test", "2026-01-31", "Active")).await["id"]);
    let fay_d = uid(&t.insert(&dispatcher, "Driver", driver("Fay", "fay@fleet.test", "2028-06-30", "Active")).await["id"]);
    let gus_d = uid(&t.insert(&dispatcher, "Driver", driver("Gus", "gus@fleet.test", "2028-06-30", "Inactive")).await["id"]);

    // ───────────── a trip planned ─────────────
    let day = "2026-11-10";
    let stop = |dn: Option<Uuid>, address: &str, lat: &str, lng: &str, kg: &str| json!({"delivery_note": dn.map(|d| d.to_string()), "address": address, "latitude": lat, "longitude": lng, "weight_kg": kg});
    let far = stop(Some(notes[0].1), "Far: 1 Airport Road", "13.1000", "77.7000", "120");
    let near = stop(Some(notes[1].1), "Near: 2 Market Street", "12.9800", "77.6000", "80");
    let mid = stop(Some(notes[2].1), "Mid: 3 Lake View", "13.0300", "77.6500", "50");
    let trip = |vehicle: Uuid, driver: Uuid, stops: Value| json!({"date": day, "depot": hub.to_string(), "vehicle": vehicle.to_string(), "driver": driver.to_string(), "stops": stops});
    refused(
        t.k.insert(&dispatcher, "Trip", obj(trip(van1, dan_d, json!([stop(Some(notes[3].1), "Dee Deli", "12.9", "77.5", "10")])))).await,
        "invalid",
        "is not submitted",
    );
    refused(t.k.insert(&dispatcher, "Trip", obj(trip(van1, dan_d, json!([near.clone(), near.clone()])))).await, "invalid", "one stop of a trip");
    let t1 = t.insert(&dispatcher, "Trip", trip(van1, dan_d, json!([far.clone(), near.clone(), mid.clone()]))).await;
    let t1_id = uid(&t1["id"]);
    assert_eq!(dec(&t1["total_weight"]), 250.0);
    let unordered = dec(&t1["distance_km"]);
    assert!(unordered > 0.0, "a distance when every stop has its place");
    let ordered = t.act(&dispatcher, t1_id, "plan_route").await.expect("ordered");
    let addresses: Vec<String> = t.stops(t1_id).await.into_iter().map(|s| s.1).collect();
    assert_eq!(addresses, ["Near: 2 Market Street", "Mid: 3 Lake View", "Far: 1 Airport Road"], "nearest first from the hub");
    assert!(dec(&ordered["distance_km"]) < unordered, "{} is shorter than {unordered}", ordered["distance_km"]);
    assert_eq!(t.stops(t1_id).await.iter().map(|s| s.0).collect::<Vec<_>>(), [1, 2, 3], "numbered in their new order");
    refused(t.k.insert(&dispatcher, "Trip", obj(trip(bike, fay_d, json!([far.clone()])))).await, "invalid", "is on TRP-");
    let nowhere = uid(&t
        .insert(
            &dispatcher,
            "Trip",
            json!({"date": day, "depot": south.to_string(), "vehicle": bike.to_string(), "driver": fay_d.to_string(),
                   "stops": [{"address": "Somewhere", "latitude": "12.9", "longitude": "77.5", "weight_kg": "5"}]}),
        )
        .await["id"]);
    assert!(t.get("Trip", nowhere).await["distance_km"].is_null(), "no distance from a depot without its place");
    refused(t.act(&dispatcher, nowhere, "plan_route").await, "invalid", "needs its latitude and longitude to order the stops");
    let unplaced = uid(&t
        .insert(
            &dispatcher,
            "Trip",
            json!({"date": day, "depot": hub.to_string(), "vehicle": bike.to_string(), "driver": fay_d.to_string(), "stops": [{"address": "Unmapped", "weight_kg": "5"}]}),
        )
        .await["id"]);
    refused(t.act(&dispatcher, unplaced, "plan_route").await, "invalid", "each stop needs its latitude and longitude");

    // ───────────── dispatching ─────────────
    let set = |id: Uuid, v: Value| {
        let k = t.k.clone();
        let d = dispatcher.clone();
        async move { k.update(&d, "Trip", id, obj(v), None).await.unwrap() }
    };
    set(t1_id, json!({"vehicle": van2.to_string()})).await;
    refused(t.act(&dispatcher, t1_id, "dispatch").await, "invalid", "VAN-2 is off the road");
    set(t1_id, json!({"vehicle": bike.to_string()})).await;
    refused(t.act(&dispatcher, t1_id, "dispatch").await, "invalid", "carries 100");
    set(t1_id, json!({"vehicle": van1.to_string(), "driver": eve_d.to_string()})).await;
    refused(t.act(&dispatcher, t1_id, "dispatch").await, "invalid", "licence is not valid on 2026-11-10");
    set(t1_id, json!({"driver": gus_d.to_string()})).await;
    refused(t.act(&dispatcher, t1_id, "dispatch").await, "invalid", "Gus is not active");
    set(t1_id, json!({"driver": dan_d.to_string()})).await;
    t.act(&dispatcher, t1_id, "dispatch").await.expect("dispatched");
    let dn = t.get("DeliveryNote", notes[0].1).await;
    assert_eq!((dn["vehicle_no"].as_str(), dn["transporter"].as_str()), (Some("VAN-1"), Some("Own fleet")), "the note carries the vehicle");
    refused(t.k.update(&dispatcher, "Trip", t1_id, obj(json!({"stops": [near.clone()]})), None).await, "invalid", "keeps its vehicle, driver and stops");
    let busy = uid(&t.insert(&dispatcher, "Trip", trip(van1, fay_d, json!([{"address": "Extra", "weight_kg": "1"}]))).await["id"]);
    refused(t.act(&dispatcher, busy, "dispatch").await, "invalid", "out on TRP-");
    let empty = uid(&t.insert(&dispatcher, "Trip", trip(bike, fay_d, json!([]))).await["id"]);
    refused(t.act(&dispatcher, empty, "dispatch").await, "invalid", "add its stops");

    // ───────────── the driver's stops, and proof of delivery ─────────────
    let mine = call(&k, &dan, "drive", json!({"day": day})).await.unwrap();
    assert_eq!(mine["trips"].as_array().unwrap().len(), 1);
    assert_eq!(mine["trips"][0]["stops"].as_array().unwrap().len(), 3, "with its stops");
    assert!(call(&k, &eve, "drive", json!({"day": day})).await.unwrap()["trips"].as_array().unwrap().is_empty(), "not hers");
    let pod = |stop: i64, outcome: &str, extra: Value| {
        let mut v = json!({"trip": t1_id.to_string(), "stop": stop, "outcome": outcome});
        v.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
        v
    };
    refused(
        t.k.insert(&eve, "ProofOfDelivery", obj(pod(1, "Delivered", json!({"received_by": "Bo", "photo": "/files/door.jpg"})))).await,
        "forbidden",
        "only the trip's driver",
    );
    refused(t.k.insert(&dan, "ProofOfDelivery", obj(pod(1, "Delivered", json!({"photo": "/files/door.jpg"})))).await, "invalid", "who took the goods");
    refused(t.k.insert(&dan, "ProofOfDelivery", obj(pod(1, "Delivered", json!({"received_by": "Bo"})))).await, "invalid", "signature or a photo");
    refused(
        t.k.insert(&dan, "ProofOfDelivery", obj(pod(9, "Delivered", json!({"received_by": "Bo", "photo": "/files/door.jpg"})))).await,
        "invalid",
        "no stop 9",
    );
    let first = t.insert(&dan, "ProofOfDelivery", pod(1, "Delivered", json!({"received_by": "Bo", "signature": "/files/sig-bo.png"}))).await;
    assert_eq!(first["recorded_by"], "dan@fleet.test");
    refused(
        t.k.insert(&dan, "ProofOfDelivery", obj(pod(1, "Delivered", json!({"received_by": "Bo", "photo": "/files/again.jpg"})))).await,
        "invalid",
        "is delivered already",
    );
    refused(
        t.k.update(&a, "ProofOfDelivery", uid(&first["id"]), obj(json!({"received_by": "Someone else"})), None).await,
        "invalid",
        "kept as it was recorded",
    );
    t.insert(&dan, "ProofOfDelivery", pod(2, "Not Home", json!({"note": "Shut for lunch"}))).await;
    let s = t.stops(t1_id).await;
    assert_eq!((s[0].2.as_str(), s[1].2.as_str(), s[2].2.as_str()), ("Delivered", "Failed", "Pending"));
    refused(t.act(&dispatcher, t1_id, "complete").await, "invalid", "[3] have no proof");
    refused(t.act(&dispatcher, t1_id, "cancel_trip").await, "invalid", "some of this trip is delivered");
    t.insert(&dispatcher, "ProofOfDelivery", pod(3, "Delivered", json!({"received_by": "Cy", "photo": "/files/cy.jpg"}))).await;
    t.act(&dispatcher, t1_id, "complete").await.expect("complete");
    let done = t.get("Trip", t1_id).await;
    assert!((dec(&done["cost"]) - dec(&done["distance_km"]) * 12.5).abs() < 0.006, "its distance at VAN-1's rate: {}", done["cost"]);
    refused(t.k.update(&dispatcher, "Trip", t1_id, obj(json!({"date": "2026-11-11"})), None).await, "invalid", "a completed trip is not changed");
    refused(
        t.k.insert(&dispatcher, "ProofOfDelivery", obj(json!({"trip": busy.to_string(), "stop": 1, "outcome": "Not Home"}))).await,
        "invalid",
        "is not out",
    );
    // Its notes are free again once it is done: another trip may take one back out.
    t.insert(
        &dispatcher,
        "Trip",
        json!({"date": "2026-11-11", "depot": hub.to_string(), "vehicle": van1.to_string(), "driver": dan_d.to_string(), "stops": [near.clone()]}),
    )
    .await;
    // A trip with nothing delivered may be cancelled.
    t.act(&dispatcher, busy, "cancel_trip").await.expect("cancelled");

    // ───────────── out again the same day ─────────────
    // VAN-1 and Dan are back: their completed trip frees them for another that day.
    let again = uid(&t.insert(&dispatcher, "Trip", trip(van1, dan_d, json!([{"address": "Back to Market Street", "weight_kg": "5"}]))).await["id"]);
    t.act(&dispatcher, again, "dispatch").await.expect("a second trip the same day");
    assert_eq!(call(&k, &dan, "drive", json!({})).await.unwrap()["day"], dan.today().to_string(), "the workspace's day by default");

    // ───────────── collections, in their windows ─────────────
    let ret = uid(&t
        .insert(
            &a,
            "DeliveryNote",
            json!({"company": company.to_string(), "customer": notes[0].0.to_string(), "set_warehouse": stores["id"], "is_return": true,
                   "return_against": notes[0].1.to_string(), "items": [{"item": parcel.to_string(), "qty": "1"}]}),
        )
        .await["id"]);
    let past = "2026-01-05";
    let collect = json!({"kind": "Collection", "return_note": ret.to_string(), "address": "Ana's Cafe, 1 Airport Road", "latitude": "13.1000", "longitude": "77.7000",
                         "weight_kg": "10", "window_from": "09:00", "window_to": "10:00"});
    let on_day = |stops: Value| json!({"date": past, "depot": hub.to_string(), "vehicle": van1.to_string(), "driver": dan_d.to_string(), "stops": stops});
    let with = |extra: Value| {
        let mut v = collect.clone();
        v.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap());
        on_day(json!([v]))
    };
    refused(t.k.insert(&dispatcher, "Trip", obj(with(json!({"delivery_note": notes[1].1.to_string()})))).await, "invalid", "its goods come with a return note");
    refused(t.k.insert(&dispatcher, "Trip", obj(with(json!({"kind": "Delivery"})))).await, "invalid", "a return note goes with a collection");
    refused(t.k.insert(&dispatcher, "Trip", obj(with(json!({"return_note": notes[3].1.to_string()})))).await, "invalid", "is a return delivery note in draft");
    refused(t.k.insert(&dispatcher, "Trip", obj(with(json!({"window_from": "9am"})))).await, "invalid", "a time like 09:30, not 9am");
    refused(t.k.insert(&dispatcher, "Trip", obj(with(json!({"window_from": "11:00"})))).await, "invalid", "closes before it opens");
    let afternoon = json!({"address": "Bo Books, 2 Market Street", "latitude": "12.9800", "longitude": "77.6000", "weight_kg": "5", "window_from": "14:00", "window_to": "16:00"});
    let anytime = json!({"address": "Cy Hardware, 3 Lake View", "latitude": "13.0300", "longitude": "77.6500", "weight_kg": "5"});
    let round = uid(&t.insert(&dispatcher, "Trip", on_day(json!([afternoon, collect.clone(), anytime]))).await["id"]);
    refused(t.k.insert(&dispatcher, "Trip", obj(on_day(json!([collect.clone()])))).await, "invalid", "is collected on TRP-");
    t.act(&dispatcher, round, "plan_route").await.expect("ordered");
    let order: Vec<String> = t.stops(round).await.into_iter().map(|s| s.1).collect();
    // The stop without a window first, then the collection opening at nine, then the afternoon's,
    // though Market Street is the nearest.
    assert_eq!(order, ["Cy Hardware, 3 Lake View", "Ana's Cafe, 1 Airport Road", "Bo Books, 2 Market Street"]);
    t.act(&dispatcher, round, "dispatch").await.expect("out on another day");
    let proof =
        |stop: i64, outcome: &str| json!({"trip": round.to_string(), "stop": stop, "outcome": outcome, "received_by": "Ana", "photo": "/files/ana.jpg"});
    refused(t.k.insert(&dan, "ProofOfDelivery", obj(proof(2, "Delivered"))).await, "invalid", "stop 2 is a collection: it is collected, not delivered");
    refused(t.k.insert(&dan, "ProofOfDelivery", obj(proof(1, "Collected"))).await, "invalid", "stop 1 is a delivery: it is delivered, not collected");
    assert_eq!(t.get("DeliveryNote", ret).await["docstatus"].as_i64(), Some(0), "in draft until the goods are collected");
    t.insert(&dan, "ProofOfDelivery", proof(2, "Collected")).await;
    assert_eq!(t.get("DeliveryNote", ret).await["docstatus"].as_i64(), Some(1), "the return note submitted: the goods are back");
    refused(t.act(&dispatcher, round, "cancel_trip").await, "invalid", "delivered or collected");
    t.insert(&dan, "ProofOfDelivery", proof(1, "Delivered")).await;
    t.insert(&dan, "ProofOfDelivery", json!({"trip": round.to_string(), "stop": 3, "outcome": "Not Home"})).await;
    let stops = t.get("Trip", round).await["stops"].as_array().unwrap().clone();
    let seen: Vec<(&str, bool)> = stops.iter().map(|s| (s["status"].as_str().unwrap(), s["late"].as_bool().unwrap_or(false))).collect();
    // Proved after its day: the stops with windows are late, the one without never is.
    assert_eq!(seen, [("Delivered", false), ("Collected", true), ("Failed", true)]);
    t.act(&dispatcher, round, "complete").await.expect("complete");

    // ───────────── fuel, services and costs ─────────────
    let v = t.k.update(&dispatcher, "Vehicle", van1, obj(json!({"odometer_km": 9000, "service_every_km": 10000, "last_service_km": 0})), None).await.unwrap();
    assert_eq!(v["service_due"], false);
    let fuel = |on: &str, litres: &str, km: i64| json!({"vehicle": van1.to_string(), "date": on, "litres": litres, "cost": format!("{litres}00"), "odometer_km": km, "driver": dan_d.to_string()});
    refused(t.k.insert(&dan, "FuelLog", obj(fuel("2026-11-01", "0", 9500))).await, "invalid", "is more than none");
    let first_fuel = uid(&t.insert(&dan, "FuelLog", fuel("2026-11-01", "40", 9500)).await["id"]);
    assert_eq!(t.get("Vehicle", van1).await["odometer_km"].as_i64(), Some(9500), "a fuel log moves the reading on");
    refused(t.k.insert(&dan, "FuelLog", obj(fuel("2026-11-02", "10", 9400))).await, "invalid", "read 9500 km by that day already");
    refused(t.k.insert(&dan, "FuelLog", obj(fuel("2026-10-30", "10", 9600))).await, "invalid", "a later fuel log reads 9500 km");
    t.insert(&dan, "FuelLog", fuel("2026-10-30", "20", 9100)).await;
    assert_eq!(t.get("Vehicle", van1).await["odometer_km"].as_i64(), Some(9500), "an earlier day's log, entered late, winds nothing back");
    t.insert(&dan, "FuelLog", fuel("2026-11-03", "45", 10100)).await;
    let v = t.get("Vehicle", van1).await;
    assert_eq!((v["odometer_km"].as_i64(), v["service_due"].as_bool()), (Some(10100), Some(true)), "due by the kilometres");
    let service = |p: Value| {
        let k = t.k.clone();
        let d = dispatcher.clone();
        async move { k.run_action(&d, "Vehicle", van1, "serviced", p).await }
    };
    refused(service(json!({"odometer_km": 10000})).await, "invalid", "reads 10100 km already");
    refused(service(json!({"odometer_km": 10150, "next_service_on": "2026-01-01"})).await, "invalid", "after today");
    service(json!({"odometer_km": "10150", "next_service_on": "2099-06-30"})).await.expect("serviced");
    let v = t.get("Vehicle", van1).await;
    assert_eq!((v["last_service_km"].as_i64(), v["odometer_km"].as_i64(), v["service_due"].as_bool()), (Some(10150), Some(10150), Some(false)));
    ferrum_app_logistics::maintenance(&k).await.expect("upkeep");
    assert_eq!(t.get("Vehicle", van1).await["service_due"].as_bool(), Some(false), "not due before its day");
    ferrum_app_logistics::maintenance_on(&k, chrono::NaiveDate::from_ymd_opt(2099, 6, 30).unwrap()).await.expect("upkeep");
    assert_eq!(t.get("Vehicle", van1).await["service_due"].as_bool(), Some(true), "due by the day");
    let range = |vehicle: Uuid| {
        HashMap::from([
            ("from_date".to_string(), "2026-01-01".to_string()),
            ("to_date".to_string(), "2026-12-31".to_string()),
            ("vehicle".to_string(), vehicle.to_string()),
        ])
    };
    let costs = ferrum_domain::reports::run(&k, &dispatcher, "logistics_fleet_costs", &range(van1)).await.expect("fleet costs");
    assert_eq!(costs["rows"].as_array().unwrap().len(), 1, "VAN-1 alone");
    let row = &costs["rows"][0];
    // Its two completed trips (the one still out is not counted), and its fuel by the odometer.
    assert_eq!((row["vehicle"].as_str(), row["trips"].as_i64(), row["driven_km"].as_i64()), (Some("VAN-1"), Some(2), Some(1000)));
    assert_eq!((dec(&row["litres"]), dec(&row["fuel_cost"]), dec(&row["litres_per_100km"])), (105.0, 10500.0, 10.5));
    let trips = [t.get("Trip", t1_id).await, t.get("Trip", round).await];
    assert!((dec(&row["trip_cost"]) - trips.iter().map(|t| dec(&t["cost"])).sum::<f64>()).abs() < 0.001, "{row}");
    assert!((dec(&row["distance_km"]) - trips.iter().map(|t| dec(&t["distance_km"])).sum::<f64>()).abs() < 0.001, "{row}");
    let idle = ferrum_domain::reports::run(&k, &dispatcher, "logistics_fleet_costs", &range(bike)).await.expect("fleet costs");
    assert_eq!((idle["rows"][0]["trips"].as_i64(), idle["rows"][0]["driven_km"].as_i64()), (Some(0), None), "nothing completed, nothing logged");

    // ───────────── who may do what ─────────────
    refused(t.k.insert(&dan, "Trip", obj(trip(van1, dan_d, json!([])))).await, "forbidden", "");
    refused(t.k.insert(&dan, "Vehicle", obj(vehicle("VAN-9", "100", "Available"))).await, "forbidden", "");
    refused(t.act(&dan, t1_id, "complete").await, "forbidden", "");
    refused(t.k.run_action(&dan, "Vehicle", van1, "serviced", json!({"odometer_km": 10200})).await, "forbidden", "");
    refused(t.k.update(&dan, "FuelLog", first_fuel, obj(json!({"litres": "1"})), None).await, "forbidden", "");
}
