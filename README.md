# Logistics

Logistics and fleets for [Ferrum](https://github.com/CBFerrum/Ferrum):
- depots, vehicles and drivers;
- trips dispatched with their stops in order: deliveries from ERP's delivery notes, and collections
  of returns, each in its window;
- proof of delivery, recorded by the driver, with or without a connection;
- the fleet's fuel, services and costs.

| | |
|---|---|
| 🚚 App | `logistics`, native, in the server |
| 🧩 Package | `logistics` (needs ERP's selling) |
| 🖥️ Interface | its own, at `/logistics/`: today's trips and the vehicles due a service for dispatchers, a driver's stops, and the records |
| 🔐 Roles | Fleet Dispatcher, Driver |
| 📜 Licence | AGPL-3.0-only |

## Highlights

- **Trips from delivery notes.** A trip's stops come with their address, place and weight, most of
  them from a submitted delivery note. A note is one stop of one open trip: a draft note, or one on
  another trip, is refused. A stop with no weight typed takes its goods': each line's quantity at
  its item's Weight (kg), the field Logistics adds to ERP's items.
- **Unknown is not nothing.** Goods whose item has no weight make the stop's and the trip's weight
  unknown, naming them, rather than counting as nothing. Such a trip is weighed before it goes:
  *Record the Weighed Load* keeps the weight, where and how, who and when, and dispatch checks the
  vehicle against it while the trip's stops are the ones weighed. A weighing lighter than the goods
  of known weight is refused.
- **Collections.** A stop may collect instead: goods coming back, with ERP's return delivery note
  for them in draft. The note is submitted when the goods are collected, so they are back in stock.
  A return note is one stop of one open trip too.
- **Windows.** A stop may have a window, from `HH:MM` to `HH:MM`. A stop proved after its window
  closed, on the workspace's clock, is marked late.
- **Stops in order.** Ordering a trip's stops visits them by when their windows open (a stop
  without one first), nearest first from the depot among those opening together. Its distance is
  the straight line from the depot through the stops and back, once every stop has its place.
- **Dispatching checks the day.** A trip is dispatched only when:
  - its vehicle is on the road and carries the load;
  - its driver is active and licensed on the day;
  - neither is out on another trip at the time: once a trip is completed, its vehicle and driver
    may go out again that day.

  Its delivery notes then carry the vehicle. A dispatched trip keeps its vehicle, driver and stops;
  a window may still move.
- **Proof of delivery.** Only the trip's driver, or a dispatcher, records what happened at a stop,
  once for each stop. A delivery names who took the goods, a collection who handed them over, and
  carries their signature or a photo; any other outcome fails the stop. The proof is kept as it was
  recorded.
- **Completing.** A trip completes once each stop is delivered, collected or failed. Its cost is
  then its distance at the vehicle's cost per km. One with nothing delivered or collected may be
  cancelled.
- **Fuel and services.** A fuel log's odometer reading never goes below one logged that day or
  before, nor above a later one, and it moves the vehicle's reading on. A vehicle's service is due
  by the day, or every so many kilometres since the last one. "It Was Serviced" records the reading
  and the next service's day. Upkeep marks services falling due by the day.
- **Fleet Costs.** A report giving each vehicle's completed trips over a range of days: their
  distance and cost, and its fuel, litres, kilometres by the odometer and litres per 100 km.
- **A driver's stops.** A driver's page lists the day's trips that are theirs, found by the address
  they signed in with. It shows each stop's window and whether it delivers or collects, records its
  proof (what happened, who, a photo of the goods or a signature drawn on the screen, a note), and
  links to logging fuel. Today's trips on the app's home link to Fleet Costs.
- **Without a connection.** The driver's page opens offline under the app's own worker, from the
  trips kept on the phone under the workspace's offline lease (Ferrum's `docs/offline.md`). A proof
  is kept on the phone for the driver alone, photo and signature and all, with the time it happened, and sent
  when the connection is back: oldest first, each safe to send twice. The server judges a stop late
  by when its proof says it happened, which may not be later than now (five minutes' grace for the
  phone's clock) nor before the trip went out. A proof the server refuses stays on the page with
  its reason until the driver discards it.

## Develop

```bash
cargo test                       # needs FERRUM_LOGISTICS_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`ui/src/trips.test.ts` covers the driver's page: what a proof needs, what is sent, kept proofs on
their stops, a lost connection told from a refusal, and the day's trips opened from the phone.
`tests/logistics_it.rs` walks a delivery day:
- a trip planned from delivery notes, its stops ordered;
- dispatch refused for each reason, then dispatched;
- proofs of delivery, and completing at the vehicle's rate;
- a driver's own stops;
- the vehicle out again the same day;
- a collection with its return note, stops ordered by their windows and proved late, proofs
  saying when they happened;
- fuel logs, services due by the kilometres and by the day, and Fleet Costs;
- who may do what.

## Not yet

- Road distances, travel times, an optimised route, geocoding addresses and live tracking: they
  need a maps provider's key (BLOCK-MAPS-KEY). Distances are straight lines, and the order is a
  sort by window, then nearest first: it does not weigh a window against the driving time.
- A service history of its own: each service shows in the vehicle's history, and its bill is an
  ERP purchase invoice. No fuel cards or telematics feeds.
- Offline, the driver's page records proofs with a photo or a signature drawn on the screen; fuel
  logs and the records pages need a connection. Photos are sent as taken.

## Changelog

| Version | Highlights |
|---|---|
| **0.4.2** | Rebuilt on the tuned design system: better menu icons, a lighter first page, a visible scroll cue, cleaner print. |
| **0.4.1** | The redesigned interface: the upgraded design system, shaped loading, told-apart states. |
| **0.4.0** | Goods without a weight are not weightless: such a trip is weighed, and the weighing recorded, before it is dispatched. |
| **0.3.12** | Its reports' names in Arabic and Hindi. |
| **0.3.11** | Its names in Arabic and Hindi; the last English sentences translated. |
| **0.3.10** | Sentences built in code are words too, in Arabic and Hindi. |
| **0.3.9** | The assistant knows its records; personal and health details in field groups. |
| **0.3.8** | Rebuilt with Ferrum-Frontend 5b22e7b: its own words in Arabic and Hindi. |
| **0.3.7** | Rebuilt with Ferrum-Frontend 6537d65: once-shown answers kept; English isolated right to left; its own words translatable by a workspace. |
| **0.3.6** | Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own interface words; pictures in attachment fields; a duplicate value said so. |
| **0.3.5** | Rebuilt with Ferrum-Frontend 63280d3: right to left, pages mirror; the records pages' words in Arabic and Hindi. |
| **0.3.4** | A stop with no weight typed takes its goods', from the Weight (kg) Logistics adds to items. |
| **0.3.3** | Rebuilt with Ferrum-Frontend a64416a: a box whose default is 1 shows ticked. |
| **0.3.2** | A signature drawn on the driver's page, kept offline with the proof; the home links Fleet Costs and shows costs, dates and readings formatted. |
| **0.3.1** | Return notes offered only where they belong; a proof sent at once says "Proof saved."; rebuilt with Ferrum-Frontend 922fcfb. |
| **0.3.0** | A driver's page that works offline: trips kept on the phone, proofs kept with when they happened and sent when the connection is back, lateness judged by that time. |
| **0.2.0** | Collections with their return notes, delivery windows and late stops, a vehicle out again the same day, fuel logs, services due, trip costs and Fleet Costs. |
| **0.1.6** | Rebuilt with Ferrum-Frontend 24c3951: times on the workspace's clock; drafts headed as drafts. |
| **0.1.5** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.1.4** | Rebuilt with Ferrum-Frontend 00dea07: a workflow state's kept fields are read-only, its needed ones required. |
| **0.1.3** | Personal-data requests find drivers by email, phone or name, and erase them. |
| **0.1.2** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Depots, vehicles and drivers, trips dispatched with their stops in order, proof of delivery and drivers' stops (FERRUM-INDUSTRY-009). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
