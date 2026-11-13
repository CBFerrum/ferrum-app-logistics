<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
  <img alt="Ferrum Logistics: your own fleet's trips, drivers' stops and proof of delivery, on ERP's delivery notes" src="docs/assets/banner-light.svg" width="100%">
</picture>

<br>

[![Version](https://img.shields.io/badge/version-0.4.2-1d4ed8?style=flat-square)](CHANGELOG.md)
[![Licence: AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue?style=flat-square)](LICENSE)
[![Ferrum platform](https://img.shields.io/badge/runs%20on-Ferrum-2f5bea?style=flat-square)](https://github.com/CBFerrum/Ferrum)
![Rust](https://img.shields.io/badge/Rust-server-b7410e?style=flat-square&logo=rust)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-data-336791?style=flat-square&logo=postgresql&logoColor=white)
![React](https://img.shields.io/badge/React-interface-0ea5e9?style=flat-square&logo=react&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-interface-3178c6?style=flat-square&logo=typescript&logoColor=white)
![Tests](https://img.shields.io/badge/tests-6%20Rust%20%C2%B7%209%20UI-16a34a?style=flat-square)

### Deliveries and collections by your own vans, from ERP's delivery notes to a signed proof, even offline.

For companies that deliver with their own vehicles: a dispatcher who plans the day, drivers with a phone, and
someone who keeps the fleet on the road. It replaces a whiteboard of trips, paper delivery slips and a fuel
spreadsheet, and it works on the same delivery notes and stock as Ferrum ERP.

[💡 Why](#-why-ferrum-logistics) · [📸 Screenshots](#-screenshots) · [✨ Features](#-features) · [📊 Compare](#-how-it-compares) · [📍 Roadmap](#-roadmap) · [🚀 Install](#-install) · [🔧 Develop](#-develop)

</div>

---

## 💡 Why Ferrum Logistics

- 🔗 **Trips from the documents you already have.** Stops come from ERP's submitted delivery notes and return
  notes, with their customer, address and weight. A collected return goes back into stock when the driver
  proves it.
- 📶 **A driver's page that works offline.** The day's trips are kept on the phone; a proof, with its photo or
  signature and the time it happened, waits there until the connection is back.
- ✅ **Dispatch that checks the day.** A trip leaves only with a vehicle on the road that carries the load, a
  driver licensed that day, and neither out on another trip.
- ⚖️ **Unknown is not nothing.** Goods without a weight make the load unknown, and such a trip is weighed and the
  weighing recorded before it goes.
- ⛽ **The fleet's costs in the same place.** Fuel logs that keep the odometer honest, services due by the day or
  the kilometres, and each vehicle's cost per trip and per 100 km.
- 🏠 **Yours to run.** Open source under the AGPL, in your own Ferrum server and PostgreSQL, with no fee per
  driver or vehicle, and REST and MCP on every record.

## 🎯 At a glance

| | |
|---|---|
| 📦 **Version** | `0.4.2` |
| 🖥️ **Kind** | Own interface at `/logistics/` (today's trips for dispatchers, a driver's stops that open offline); every other record in Ferrum's records frame |
| 🧱 **What it adds** | 1 package; 6 entities and 1 line table; 2 roles; 1 report; 1 menu; a Weight (kg) field on items |
| 🔗 **Depends on** | Ferrum platform `>=0.3.2, <0.4` and ERP (customers, items, delivery notes) |
| 🤝 **Works with** | ERP's delivery notes, return notes and stock |
| 🗣️ **Languages** | English, Arabic and Hindi; pages mirror right to left |
| 🧪 **Tests** | 6 Rust tests against a real PostgreSQL that walk a whole delivery day, 9 interface tests |
| 📄 **Licence** | AGPL-3.0-only |

## 📸 Screenshots

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-trip-list-dark.webp">
  <img alt="Trips" src="docs/assets/screens/logistics-trip-list-light.webp" width="100%">
</picture>
<p align="center"><b>🗂️ Trips</b><br><sub>Browse trips by day, vehicle, driver and status.</sub></p>

<table>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-trip-detail-dark.webp"><img alt="A trip" src="docs/assets/screens/logistics-trip-detail-light.webp"></picture>
<p><b>🚚 A trip</b><br><sub>Depot, vehicle, driver and stops in order, with the load, the distance and the actions to order the stops and record the weighed load.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-home-dark.webp"><img alt="Today's trips for dispatchers" src="docs/assets/screens/logistics-home-light.webp"></picture>
<p><b>🏠 Today's trips</b><br><sub>The dispatcher's day: each trip with its load, distance and stops done, the vehicles whose service is due, and links to a new trip, your stops and fleet costs.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-your-stops-dark.webp"><img alt="Your stops" src="docs/assets/screens/logistics-your-stops-light.webp"></picture>
<p><b>📱 Your stops</b><br><sub>The driver's view: the stops of today's trip in order, a proof at each, and fuel one tap away.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-vehicle-list-dark.webp"><img alt="Vehicles" src="docs/assets/screens/logistics-vehicle-list-light.webp"></picture>
<p><b>🚐 Vehicles</b><br><sub>The fleet with capacity, status, odometer and whether a service is due.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-driver-list-dark.webp"><img alt="Drivers" src="docs/assets/screens/logistics-driver-list-light.webp"></picture>
<p><b>🧑‍✈️ Drivers</b><br><sub>Drivers with their sign-in, licence expiry and status.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-depot-list-dark.webp"><img alt="Depots" src="docs/assets/screens/logistics-depot-list-light.webp"></picture>
<p><b>🏭 Depots</b><br><sub>Where trips start and end.</sub></p>
</td>
</tr>
</table>

<p align="center">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-home-phone-dark.webp"><img alt="Today's trips on a phone" src="docs/assets/screens/logistics-home-phone-light.webp" width="260"></picture>
&nbsp;&nbsp;
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/logistics-trip-list-phone-dark.webp"><img alt="Trips on a phone" src="docs/assets/screens/logistics-trip-list-phone-light.webp" width="260"></picture>
</p>
<p align="center"><sub>📱 Today's trips and the trip list on a phone.</sub></p>

> [!TIP]
> The screenshots follow your GitHub theme: switch between light and dark to see both.

## ✨ Features

### 🏭 Depots, vehicles and drivers

- **Depots** with an address and a position, where trips start and end.
- **Vehicles** by registration and type (van, truck, car, bike), with their depot, capacity in kilograms, status
  (Available or Off Road), odometer, service interval and cost per kilometre.
- **Drivers** with their name, sign-in address, phone, licence number and expiry, and status. The licence number
  is in the identity field group, seen only by roles granted it, and sent to an AI provider only where the
  workspace allows that group.

### 🚚 Trips and stops

- **A trip** is a vehicle and its driver out for a day from a depot, numbered `TRP-{YYYY}-{seq:5}`, with its stops
  in order: Planned, Dispatched, Completed or Cancelled.
- **Deliveries** come from submitted ERP delivery notes, each bringing its customer, address and goods. A note is
  one stop of one open trip: a draft note, or one on another trip, is refused.
- **Collections** bring goods back: a stop names ERP's return delivery note in draft, which is submitted when the
  goods are collected, so they are back in stock.
- **Windows** from `HH:MM` to `HH:MM`; a stop proved after its window closed, on the workspace's clock, is marked
  late.
- **Weight**: a stop with no weight typed takes its goods', each line's quantity at its item's Weight (kg). The
  trip's load is the sum, collections included.
- **Order the Stops** visits them by when their windows open (a stop without one first), nearest first from the
  depot among those opening together. The trip's distance is the straight line from the depot through the stops
  and back, once every stop has its place.

### ⚖️ Unknown weights and the weighed load

- Goods whose item has no weight make the stop's and the trip's weight **unknown**, naming the goods, rather than
  counting as nothing.
- Such a trip is weighed before it goes: **Record the Weighed Load** keeps the weight, where and how it was weighed,
  who and when. Dispatch checks the vehicle against it while the trip's stops are the ones weighed.
- A weighing lighter than the goods of known weight is refused, and drivers do not record it.

### ✅ Dispatch and completion

- **Dispatch It** succeeds only when the vehicle is on the road and carries the load, the driver is active and
  licensed on the day, neither is out on another trip, every delivery note is submitted and every return note is
  in draft, each on no other open trip. A refusal names the reason, with figures ("carries 100 kg").
- **After dispatch** the delivery notes carry the vehicle's registration; the trip keeps its vehicle, driver and
  stops, though a window may still move.
- **Same-day reuse**: once a trip is completed, its vehicle and driver may go out again that day.
- **Complete It** once each stop is delivered, collected or failed; the trip's cost is then its distance at the
  vehicle's cost per kilometre.
- **Cancel It** for a trip with nothing delivered or collected.

### ✍️ Proof of delivery

- Only the trip's driver, or a dispatcher, records what happened at a stop, **once for each stop**.
- **Outcomes**: Delivered, Collected, Refused, Damaged, Not Home. A delivery names who took the goods, a collection
  who handed them over, and carries their signature or a photo; any other outcome fails the stop.
- **When it happened** comes from the driver's phone, so a proof sent later keeps its time: not later than now
  (five minutes' grace for the phone's clock) and not before the trip went out. Lateness is judged by that time.
- A proof keeps its position, a note, and who recorded it, and is kept as it was recorded.

### 📱 The driver's page

- **Your stops** lists the day's dispatched trips that belong to the signed-in driver, found by the address they
  signed in with; a driver reads no other driver's record or trips this way.
- Each stop shows its address, window and whether it delivers or collects, and records its proof: what happened,
  who, a photo of the goods or a signature drawn on the screen, and a note.
- **Offline**: the page opens without a connection under the app's own worker, from the trips kept on the phone
  under the workspace's offline lease. A proof is kept on the phone for the driver alone, photo and signature and
  all, and sent when the connection is back, oldest first, each safe to send twice. A proof the server refuses
  stays on the page with its reason until the driver discards it.
- **Log fuel** is one tap away (it needs a connection).

### ⛽ Fuel, services and costs

- **Fuel logs** record litres, cost, the odometer and the driver. A reading is never below one logged that day or
  before, nor above a later one, and it moves the vehicle's odometer on, never back.
- **Services** fall due by the day or every so many kilometres since the last one; **It Was Serviced** records the
  reading and the next service's day. The daily upkeep marks services falling due by the day.
- **Fleet Costs** report: each vehicle's completed trips over a range of days, their distance and cost, and its fuel,
  litres, kilometres by the odometer and litres per 100 km.

### 🖥️ Its own screens

| Screen | Who | What they do there |
|---|---|---|
| 🏠 Today's trips | Dispatchers | Each trip with its load, distance, stops done and cost; trips whose weight is unknown and not yet weighed; vehicles whose service is due |
| 📱 Your stops | Drivers | Today's stops in order, proofs with photo or signature, fuel |

Every other record (trips, proofs, vehicles, fuel logs, drivers, depots) opens in Ferrum's records frame with
lists, forms, search, filters, exports and history.

### 🔐 Roles, people and languages

- **Fleet Dispatcher** plans and dispatches trips, keeps the fleet and its drivers, and sees every proof of
  delivery. **Driver** reads trips, vehicles and depots, and records proofs and fuel.
- **Personal data**: a person's request finds a driver by email, phone or name, and an erasure takes their name,
  contact and licence details away.
- **The assistant** knows how trips, stops, proofs and fuel work in these pages and suggests questions to start
  with ("Which stops failed this week, and why?").
- **English, Arabic and Hindi**, pages mirrored right to left; a workspace can use its own words.

## 🧭 How it works

### 🔁 A delivery day

```mermaid
flowchart LR
    DN["ERP delivery notes"] --> T["Trip planned"]
    RN["ERP return notes, draft"] --> T
    T --> O["Stops ordered"]
    O --> W{"Weight known?"}
    W -->|no| WL["Weighed load recorded"]
    W -->|yes| D["Dispatched"]
    WL --> D
    D --> P["Proof at each stop"]
    P --> C["Completed, cost booked on the trip"]
    P --> S[("ERP stock: returns back in")]
```

A dispatcher builds the trip from delivery and return notes, orders the stops and, if some goods have no weight,
records the weighed load. Dispatch checks the vehicle and driver for the day. The driver proves each stop from the
phone; a collection submits its return note, and the trip completes when every stop is done.

### 🧩 Inside Ferrum

```mermaid
flowchart TD
    DRV["Driver's phone: /logistics/drive"] --> SW["App worker and offline store"]
    SW --> API["Ferrum REST and MCP"]
    DSP["Dispatcher: /logistics/"] --> API
    API --> LOG["Logistics engine: trips, proofs, fleet"]
    LOG --> ERP["ERP: delivery notes, return notes, items"]
    LOG --> DB[("PostgreSQL")]
```

The driver's page is served with its own worker, so it opens without a network; the server judges every proof when
it arrives. Logistics reads and updates ERP's delivery notes through the platform's records, so stock and billing
stay ERP's.

## 🧱 Data model

| Entity | What it is | Key fields or states |
|---|---|---|
| Depot | Where trips start and end | address, latitude, longitude |
| Vehicle | One vehicle of the fleet | type, capacity (kg), Available or Off Road, odometer, service every (km), cost per km, service due |
| Driver | A person who drives | email (sign-in), licence number and expiry, Active or Inactive |
| Trip | A vehicle and driver out for a day | Planned, Dispatched, Completed, Cancelled; load, weighed load, distance, cost |
| Stop (line of a trip) | A delivery or a collection | delivery or return note, window, weight, Pending, Delivered, Collected, Failed, late |
| Proof of Delivery | What happened at a stop | Delivered, Collected, Refused, Damaged, Not Home; who, signature, photo, when |
| Fuel Log | Fuel put in a vehicle | litres, cost, odometer, driver |

| Role | What it can do |
|---|---|
| Fleet Dispatcher | Plans and dispatches trips, keeps vehicles, drivers and depots, sees every proof of delivery |
| Driver | Reads trips, vehicles and depots; records proofs of delivery and fuel logs |

**Report**: Fleet Costs. **Scheduled work**: the upkeep marks vehicles whose service falls due by the day.

## 🔌 Integrations

| With | How |
|---|---|
| 📦 ERP delivery notes | A trip's deliveries; after dispatch they carry the vehicle's registration and "Own fleet" as transporter |
| ↩️ ERP return notes | A collection's draft return note, submitted when the goods are collected, so they are back in stock |
| 🏷️ ERP items | A Weight (kg) field that stops take their weight from |
| 📶 Ferrum offline leases | Which trips a driver's phone may keep for working offline |
| 🔌 REST and MCP | Every record, under each person's permissions |

Nothing outside Ferrum is needed. Distances are straight lines worked out on the server, so no maps key is used.

## 📊 How it compares

Compared with Onfleet and Routific (last-mile delivery), Fleetio (fleet maintenance), Odoo (Inventory and Fleet)
and Oracle Transportation Management, the tools a company with its own delivery fleet usually weighs.

| Capability | Ferrum Logistics | Onfleet | Routific | Fleetio | Odoo | Oracle TM |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Depots, vehicles and drivers | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Trips built from the ERP's delivery documents | ✅ | 🟡 | 🟡 | ➖ | 🟡 | ✅ |
| Collections and returns on the same trip | ✅ | ✅ | ✅ | ➖ | 🟡 | ✅ |
| Delivery windows and late stops | ✅ | ✅ | ✅ | ➖ | 🟡 | ✅ |
| Route optimisation on real roads | ❌ | ✅ | ✅ | ❌ | 🟡 | ✅ |
| Road distances, ETAs and geocoding | ❌ | ✅ | ✅ | ❌ | 🟡 | ✅ |
| Live vehicle tracking | ❌ | ✅ | ✅ | 🟡 | 🟡 | ✅ |
| Capacity checked before dispatch | ✅ | ✅ | ✅ | ➖ | 🟡 | ✅ |
| Load by volume and pallets, not only weight | ❌ | 🟡 | 🟡 | ➖ | 🟡 | ✅ |
| Driver's licence and availability checked on dispatch | ✅ | 🟡 | 🟡 | 🟡 | 🟡 | 🟡 |
| Driver's app with the day's stops | ✅ | ✅ | ✅ | 🟡 | 🟡 | 🟡 |
| Proof of delivery with signature or photo | ✅ | ✅ | ✅ | ➖ | 🟡 | 🟡 |
| Returns put back in stock by the proof | ✅ | ❌ | ❌ | ➖ | 🟡 | 🟡 |
| Customer told: out for delivery, ETA, delivered | ❌ | ✅ | ✅ | ➖ | 🟡 | 🟡 |
| Fuel logs and cost per kilometre | ✅ | ❌ | ❌ | ✅ | 🟡 | ❌ |
| Fuel cards and telematics feeds | ❌ | ❌ | ❌ | ✅ | 🟡 | 🟡 |
| Service reminders by date or distance | ✅ | ❌ | ❌ | ✅ | 🟡 | ❌ |
| Service history and work orders | 🟡 ¹ | ❌ | ❌ | ✅ | ✅ | ❌ |
| Freight cost posted to the books | ❌ | ❌ | ❌ | 🟡 | 🟡 | ✅ |
| Outside carriers, tendering and multi-leg loads | 🟡 ² | ❌ | ❌ | ❌ | 🟡 | ✅ |
| Same records as the ERP and its stock | ✅ | ❌ | ❌ | ❌ | ✅ | 🟡 |
| Self-hosted on your own servers | ✅ | ❌ | ❌ | ❌ | ✅ | 🟡 |
| Open source | ✅ | ❌ | ❌ | ❌ | 🟡 | ❌ |
| No fee per driver or vehicle | ✅ | ❌ | ❌ | ❌ | 🟡 | ❌ |

<sub>¹ Services are recorded on the vehicle and show in its history; their bills are ERP purchase invoices. ² Parcel
carriers are ERP's carrier connections (rates, labels, tracking); no tendering or multi-leg planning. ➖ means the
product does not aim at it. For the leaders, 🟡 means partly, or through another edition, module or partner app.</sub>

### 🏆 Where Ferrum Logistics stands out

- 🔗 Trips are ERP's own delivery and return notes, and a proved collection puts goods back in stock.
- 📶 A driver's page that keeps working without a connection, with proofs that keep the time they happened.
- ⚖️ Dispatch that refuses an unknown load, an unlicensed driver or a vehicle already out.
- ⛽ Delivery and fleet costs in one app, beside the books.
- 🏠 Open source, self-hosted, no fee per driver or vehicle.

### 🚧 Where the leaders are still ahead

- 🗺️ Route optimisation on real roads, ETAs, geocoding and live tracking.
- 📣 Customer notifications with an ETA and a tracking link.
- 🚛 Load planning by volume and pallets, outside carriers, tendering and multi-leg transport.
- 🔧 Telematics, fuel cards and a full maintenance history.

> [!NOTE]
> Leader columns are from public product information as of November 2026 and may change.

## 📍 Roadmap

### ✅ Recently shipped

- [x] **0.4.2** Rebuilt on the tuned design system: better menu icons, a lighter first page, cleaner print.
- [x] **0.4.1** The redesigned interface: the home on the kit's work-home pattern, shaped loading, told-apart states.
- [x] **0.4.0** Goods without a weight are not weightless: such a trip is weighed and recorded before dispatch.
- [x] **0.3.12** Reports' names and parameters in Arabic and Hindi.
- [x] **0.3.11** Models, fields and the manifest in Arabic and Hindi.
- [x] **0.3.9** The assistant knows its records; the driver's licence number in the identity field group.

### 🔜 Next up

**Routes**
- [ ] Road distances, travel times and an optimised order that weighs windows against driving time, through a
  self-hosted routing engine.
- [ ] Addresses geocoded, so stops get their position without typing it.
- [ ] Live tracking of dispatched trips.

**Customers and the books**
- [ ] Customers told when their goods are out for delivery, with an ETA, and sent the proof.
- [ ] A trip's cost posted to the ledger and spread over the orders or customers it served.

### 🔭 Later

- [ ] Load by volume and pallets, with one place for an item's weight and size shared with the warehouse.
- [ ] Inbound trips, outside carriers, tendering and multi-leg transport.
- [ ] A service history of its own, fuel cards and telematics feeds.
- [ ] Fuel logs and the records pages offline too, and photos made smaller before they are sent.

## 🚀 Install

Logistics ships in the official Ferrum server image; an administrator installs it from **Apps** (`/store/`).
Installing it installs ERP whole if the workspace does not have it yet.

First steps:

1. Make the **depots**, with their positions if you want distances and ordered stops.
2. Add the **vehicles** (capacity, cost per km, service interval) and the **drivers** with the address they sign in
   with and their licence expiry.
3. Give dispatchers **Fleet Dispatcher** and drivers **Driver**; drivers open **Your stops** on their phone once
   while online, so it is kept for offline use.
4. Fill **Weight (kg)** on the items you deliver, so loads are known without weighing.
5. Plan a trip from submitted delivery notes, order its stops and dispatch it.

## 🔧 Develop

Clone it next to the platform:

```text
work/
├── ferrum/                https://github.com/CBFerrum/Ferrum (with Ferrum-Frontend cloned into ferrum/web)
└── ferrum-app-logistics/
```

To use a local platform checkout instead of GitHub's, add a git-ignored `.cargo/config.toml` with
`[patch."https://github.com/CBFerrum/Ferrum"]` entries for `ferrum-kernel`, `ferrum-meta`, `ferrum-domain` and
`ferrum-apps` pointing at `../ferrum/crates/<name>`; ERP, which the tests use, is taken from GitHub unless you patch
`https://github.com/CBFerrum/ferrum-app-erp` the same way.

### 🧪 Tests

The tests need PostgreSQL with an ordinary (non-superuser) role, so row-level security applies:

```bash
createuser ferrum --pwprompt
createdb -O ferrum ferrum_logistics_test
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`FERRUM_LOGISTICS_TEST_DATABASE_URL` overrides the address (default
`postgres://ferrum:ferrum@localhost:5432/ferrum_logistics_test`). `tests/logistics_it.rs` walks a delivery day:

- a trip planned from delivery notes, its stops ordered;
- dispatch refused for each reason, then dispatched;
- proofs of delivery, and completing at the vehicle's rate;
- a driver's own stops, and the vehicle out again the same day;
- a collection with its return note, stops ordered by their windows and proved late, proofs saying when they happened;
- fuel logs, services due by the kilometres and by the day, and Fleet Costs;
- who may do what.

### 🖥️ Interface

The interface lives in `ui/` and builds with Ferrum-Frontend's packages from `../ferrum/web`:

```bash
cd ui
npm run check
npm run typecheck
npm test           # the driver's page: what a proof needs, kept proofs, a lost connection told from a refusal
npm run build      # into ui/dist
```

## 📂 Project layout

```text
app.yaml          the manifest: requirements, launcher entry, Create menu, assistant
packages/         the logistics package: depots, vehicles, drivers, trips and stops, proofs, fuel logs
src/
├── trips.rs      planning, ordering, weighing, dispatch, completion
├── pod.rs        proof of delivery, lateness, the phone's clock
├── fleet.rs      fuel, services and Fleet Costs
├── geo.rs        straight-line distances and nearest-first order
└── site.rs       the driver's endpoint
ui/               its own interface (React + Vite); ui/public/sw.js is the offline worker
tests/            a delivery day against a real PostgreSQL
docs/             provenance
```

## 📜 Changelog

Every release is in [CHANGELOG.md](CHANGELOG.md).

## 🤝 Contributing

Issues and pull requests are welcome at [github.com/CBFerrum/ferrum-app-logistics](https://github.com/CBFerrum/ferrum-app-logistics/issues).
Run the checks in [Develop](#-develop) first; [docs/PROVENANCE.md](docs/PROVENANCE.md) says where the app's ideas
come from.

## 📄 License

[AGPL-3.0-only](LICENSE).

---

<div align="center">
<sub>Part of <a href="https://github.com/CBFerrum/Ferrum">Ferrum</a>, the open business platform · Made with 🦀 Rust and 🐘 PostgreSQL</sub>
</div>
