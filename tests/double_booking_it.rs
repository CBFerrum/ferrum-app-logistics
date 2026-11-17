//! Double-booking under load against a real PostgreSQL (`FERRUM_LOGISTICS_TEST_DATABASE_URL`,
//! default `postgres://ferrum:ferrum@localhost:5432/ferrum_logistics_test`; reset first): fifty
//! requests for one vehicle or one driver on one day, or for one delivery or return note, at the
//! same moment, each in its own transaction on its own connection, and exactly one of them gets it.

use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// How many ask at once.
const AT_ONCE: usize = 50;

/// Each race is run on three days: once alone, the first request may by chance finish before the
/// others look.
const DAYS: [&str; 3] = ["2026-11-10", "2026-11-11", "2026-11-12"];

/// One database: the tests here take turns, each resetting it.
static TURN: Mutex<()> = Mutex::new(());

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn run(scenario: impl Future<Output = ()> + Send + 'static) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(AT_ONCE).enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario).await.unwrap() });
}

struct Yard {
    k: Arc<Kernel>,
    admin: Ctx,
    company: String,
    stores: Value,
    customer: String,
    item: String,
    depot: String,
}

impl Yard {
    /// A company with a depot, a customer and a hamper to deliver.
    async fn new() -> Yard {
        let url =
            std::env::var("FERRUM_LOGISTICS_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_logistics_test".into());
        // A connection for each request, and one for each side transaction a number takes.
        let db = Db::connect(DbConfig { url, max_connections: 2 * AT_ONCE as u32 + 8, ..Default::default() }).await.expect("connect");
        sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
        let mut packages = ferrum_domain::packages().expect("platform packages");
        packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
        packages.extend(ferrum_app_logistics::packages().expect("app packages"));
        let mut hooks = HookRegistry::default();
        ferrum_domain::register(&mut hooks);
        hooks.app("erp", &[], ferrum_app_erp::register);
        ferrum_app_logistics::register(&mut hooks);
        let k = Kernel::boot(db, Catalog::from_packages(packages).expect("catalog"), hooks, KernelSettings::default(), None).await.expect("boot");
        ferrum_domain::maintenance(&k).await.expect("partitions");
        ferrum_app_erp::maintenance(&k).await.expect("partitions");
        let tenant = k
            .create_tenant(NewTenant {
                slug: "yard".into(),
                name: "Yard Foods".into(),
                admin_email: "admin@yard.test".into(),
                admin_password: "Correct-Horse-9".into(),
                admin_name: None,
                packages: vec!["logistics".into()],
                settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
            })
            .await
            .expect("tenant");
        let session = k.login("yard", "admin@yard.test", "Correct-Horse-9").await.expect("login");
        let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
        let mut y = Yard { k, admin, company: String::new(), stores: Value::Null, customer: String::new(), item: String::new(), depot: String::new() };
        let inr = y.find("Currency", json!({"code": "INR"})).await.remove(0)["id"].clone();
        y.company = y.insert("Company", json!({"company_name": "Yard Foods", "abbr": "YF", "default_currency": inr})).await;
        y.stores = y.find("Warehouse", json!({"warehouse_name": "Stores", "company": y.company})).await.remove(0)["id"].clone();
        let services = y.find("ItemGroup", json!({"group_name": "Services"})).await.remove(0)["id"].clone();
        let nos = y.find("Uom", json!({"uom_name": "Nos"})).await.remove(0)["id"].clone();
        y.item =
            y.insert("Item", json!({"item_code": "HAMPER", "item_name": "Hamper", "item_group": services, "stock_uom": nos, "is_stock_item": false})).await;
        y.customer = y.insert("Customer", json!({"customer_name": "Ana's Cafe", "customer_type": "Company"})).await;
        y.depot = y.insert("Depot", json!({"depot_name": "North Hub"})).await;
        y
    }

    async fn insert(&self, entity: &str, v: Value) -> String {
        let doc = self.k.insert(&self.admin, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"));
        doc["id"].as_str().unwrap().to_string()
    }

    async fn find(&self, entity: &str, filters: Value) -> Vec<Doc> {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(500), ..Default::default() };
        self.k.list(&self.admin, entity, &q).await.unwrap().rows
    }

    async fn vehicle(&self, registration: &str) -> String {
        self.insert("Vehicle", json!({"registration": registration, "capacity_kg": "500", "depot": self.depot, "status": "Available"})).await
    }

    async fn driver(&self, name: &str) -> String {
        self.insert("Driver", json!({"full_name": name, "licence_number": format!("DL-{name}"), "licence_expires_on": "2027-12-31", "status": "Active"})).await
    }

    fn trip(&self, day: &str, vehicle: &str, driver: &str, stops: Value) -> Value {
        json!({"date": day, "depot": self.depot, "vehicle": vehicle, "driver": driver, "stops": stops})
    }

    /// A planned trip of `vehicle` and `driver` on `day`, with one stop.
    async fn planned(&self, day: &str, vehicle: &str, driver: &str) -> String {
        self.insert("Trip", self.trip(day, vehicle, driver, json!([{"address": "1 Market Street", "weight_kg": "5"}]))).await
    }

    /// A submitted delivery note of one hamper.
    async fn delivery_note(&self) -> String {
        let dn = self
            .insert(
                "DeliveryNote",
                json!({"company": self.company, "customer": self.customer, "set_warehouse": self.stores, "items": [{"item": self.item, "qty": "1", "rate": "500"}]}),
            )
            .await;
        self.k.submit(&self.admin, "DeliveryNote", dn.parse().unwrap(), None).await.expect("submitted");
        dn
    }

    /// Saving `v` as a new `entity`, when awaited.
    fn save(&self, entity: &'static str, v: Value) -> impl Future<Output = Result<Doc, Error>> + Send + 'static {
        let (k, admin) = (self.k.clone(), self.admin.clone());
        async move { k.insert(&admin, entity, obj(v)).await }
    }

    /// Dispatching `trip`, when awaited.
    fn dispatch(&self, trip: &str) -> impl Future<Output = Result<Value, Error>> + Send + 'static {
        let (k, admin, trip) = (self.k.clone(), self.admin.clone(), trip.parse::<Uuid>().unwrap());
        async move { k.run_action(&admin, "Trip", trip, "dispatch", Value::Null).await }
    }

    /// `calls`, released together, each in its own task: how many succeeded, and the refusals.
    async fn rush<T: Send + 'static>(&self, calls: Vec<impl Future<Output = Result<T, Error>> + Send + 'static>) -> (usize, Vec<Error>) {
        // Each one's connection open before they start, so none waits for another to be made.
        let open: Vec<_> = (0..calls.len()).map(|_| tokio::spawn(self.k.db.pool.acquire())).collect();
        for c in open {
            drop(c.await.unwrap().expect("a connection"));
        }
        let gate = Arc::new(tokio::sync::Barrier::new(calls.len()));
        let tasks: Vec<_> = calls
            .into_iter()
            .map(|call| {
                let gate = gate.clone();
                tokio::spawn(async move {
                    gate.wait().await;
                    call.await
                })
            })
            .collect();
        let (mut done, mut refused) = (0, Vec::new());
        for t in tasks {
            match t.await.unwrap() {
                Ok(_) => done += 1,
                Err(e) => refused.push(e),
            }
        }
        (done, refused)
    }
}

#[test]
fn fifty_trips_of_one_vehicle_on_one_day_dispatch_one() {
    run(async {
        let y = Yard::new().await;
        let van = y.vehicle("VAN-1").await;
        let mut drivers = Vec::new();
        for i in 0..AT_ONCE {
            drivers.push(y.driver(&format!("Driver {i}")).await);
        }
        for day in DAYS {
            let mut trips = Vec::new();
            for driver in &drivers {
                trips.push(y.planned(day, &van, driver).await);
            }
            let (sent, refused) = y.rush(trips.iter().map(|t| y.dispatch(t)).collect()).await;
            assert_eq!(sent, 1, "{AT_ONCE} trips of one vehicle on {day}, dispatched at once: {sent} went out");
            assert!(refused.iter().all(|e| format!("{e:?}").contains("is out on")), "refused because the vehicle is out: {refused:?}");
            assert_eq!(y.find("Trip", json!({"vehicle": van, "date": day, "status": "Dispatched"})).await.len(), 1);
        }
    });
}

#[test]
fn fifty_trips_of_one_driver_on_one_day_dispatch_one() {
    run(async {
        let y = Yard::new().await;
        let dan = y.driver("Dan").await;
        let mut vans = Vec::new();
        for i in 0..AT_ONCE {
            vans.push(y.vehicle(&format!("VAN-{i}")).await);
        }
        for day in DAYS {
            let mut trips = Vec::new();
            for van in &vans {
                trips.push(y.planned(day, van, &dan).await);
            }
            let (sent, refused) = y.rush(trips.iter().map(|t| y.dispatch(t)).collect()).await;
            assert_eq!(sent, 1, "{AT_ONCE} trips of one driver on {day}, dispatched at once: {sent} went out");
            assert!(refused.iter().all(|e| format!("{e:?}").contains("is out on")), "refused because the driver is out: {refused:?}");
            assert_eq!(y.find("Trip", json!({"driver": dan, "date": day, "status": "Dispatched"})).await.len(), 1);
        }
    });
}

#[test]
fn fifty_trips_with_one_delivery_note_plan_it_once() {
    run(async {
        let y = Yard::new().await;
        let (van, dan) = (y.vehicle("VAN-1").await, y.driver("Dan").await);
        for day in DAYS {
            let dn = y.delivery_note().await;
            let trip = y.trip(day, &van, &dan, json!([{"delivery_note": dn, "address": "Ana's Cafe, 1 Airport Road", "weight_kg": "5"}]));
            let (saved, refused) = y.rush((0..AT_ONCE).map(|_| y.save("Trip", trip.clone())).collect()).await;
            assert_eq!(saved, 1, "{AT_ONCE} trips with one delivery note, saved at once: {saved} were saved");
            assert!(refused.iter().all(|e| format!("{e:?}").contains("is on TRP-")), "refused because the note is on a trip: {refused:?}");
            assert_eq!(y.find("Trip", json!({"date": day})).await.len(), 1);
        }
    });
}

#[test]
fn fifty_trips_collecting_one_return_note_plan_it_once() {
    run(async {
        let y = Yard::new().await;
        let (van, dan) = (y.vehicle("VAN-1").await, y.driver("Dan").await);
        for day in DAYS {
            let dn = y.delivery_note().await;
            let back = y
                .insert(
                    "DeliveryNote",
                    json!({"company": y.company, "customer": y.customer, "set_warehouse": y.stores, "is_return": true, "return_against": dn,
                           "items": [{"item": y.item, "qty": "1"}]}),
                )
                .await;
            let trip = y.trip(day, &van, &dan, json!([{"kind": "Collection", "return_note": back, "address": "Ana's Cafe, 1 Airport Road", "weight_kg": "5"}]));
            let (saved, refused) = y.rush((0..AT_ONCE).map(|_| y.save("Trip", trip.clone())).collect()).await;
            assert_eq!(saved, 1, "{AT_ONCE} trips collecting one return note, saved at once: {saved} were saved");
            assert!(refused.iter().all(|e| format!("{e:?}").contains("is collected on TRP-")), "refused because the note is on a trip: {refused:?}");
            assert_eq!(y.find("Trip", json!({"date": day})).await.len(), 1);
        }
    });
}
