# Logistics

Logistics and fleets for [Ferrum](https://github.com/CBFerrum/Ferrum):
- depots, vehicles and drivers;
- trips dispatched with their stops in order, from ERP's delivery notes;
- proof of delivery, recorded by the driver.

| | |
|---|---|
| 🚚 App | `logistics`, native, in the server |
| 🧩 Package | `logistics` (needs ERP's selling) |
| 🖥️ Interface | its own, at `/logistics/`: today's trips for dispatchers, a driver's stops, and the records |
| 🔐 Roles | Fleet Dispatcher, Driver |
| 📜 Licence | AGPL-3.0-only |

## Highlights

- **Trips from delivery notes.** A trip's stops come with their address, place and weight, most of
  them from a submitted delivery note. A note is one stop of one open trip: a draft note, or one on
  another trip, is refused.
- **Stops in order.** Ordering a trip's stops visits the nearest first from its depot. Its distance
  is the straight line from the depot through the stops and back, once every stop has its place.
- **Dispatching checks the day.** A trip is dispatched only when:
  - its vehicle is on the road and carries the load;
  - its driver is active and licensed on the day;
  - neither is out on another trip that day.

  Its delivery notes then carry the vehicle. A dispatched trip keeps its vehicle, driver and stops.
- **Proof of delivery.** Only the trip's driver, or a dispatcher, records what happened at a stop,
  once for each stop. A delivery names who took the goods and carries their signature or a photo;
  any other outcome fails the stop. The proof is kept as it was recorded.
- **Completing.** A trip completes once each stop is delivered or failed. One with nothing
  delivered may be cancelled.
- **A driver's stops.** A driver's page lists the day's trips that are theirs, found by the address
  they signed in with, and opens each stop's proof already filled in.

## Develop

```bash
cargo test                       # needs FERRUM_LOGISTICS_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`tests/logistics_it.rs` walks a delivery day:
- a trip planned from delivery notes, its stops ordered;
- dispatch refused for each reason, then dispatched;
- proofs of delivery, and completing;
- a driver's own stops;
- who may do what.

## Not yet

- Road distances, travel times, an optimised route, geocoding addresses and live tracking: they
  need a maps provider's key (BLOCK-MAPS-KEY). Distances are straight lines and the order is the
  nearest first.
- Delivery windows, several trips a vehicle a day, and returns collected on the way.
- Fuel, maintenance schedules and costs per kilometre.
- A driver's app that works offline.

## Changelog

| Version | Highlights |
|---|---|
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Depots, vehicles and drivers, trips dispatched with their stops in order, proof of delivery and drivers' stops (FERRUM-INDUSTRY-009). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
