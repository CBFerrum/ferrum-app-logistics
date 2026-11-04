//! A delivery day (FERRUM-INDUSTRY-009) against a real PostgreSQL
//! (`FERRUM_LOGISTICS_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_logistics_test`;
//! reset first): a trip planned from submitted delivery notes, its stops ordered nearest first from
//! the depot; dispatch refused for a vehicle off the road or too small, a driver unlicensed or
//! inactive, and a vehicle already out; the notes carrying the vehicle; proofs of delivery recorded
//! only by the trip's driver, once a stop, with who took the goods and their signature or a photo;
//! completing and cancelling; a driver's own stops; and who may do what.

use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
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

    // ───────────── who may do what ─────────────
    refused(t.k.insert(&dan, "Trip", obj(trip(van1, dan_d, json!([])))).await, "forbidden", "");
    refused(t.k.insert(&dan, "Vehicle", obj(vehicle("VAN-9", "100", "Available"))).await, "forbidden", "");
    refused(t.act(&dan, t1_id, "complete").await, "forbidden", "");
}
