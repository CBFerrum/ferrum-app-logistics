# Changelog

## 0.1.1

Rebuilt with Ferrum-Frontend 80ff179: a field's label names its control for assistive technology, and a record whose title is a link shows that record's title, not its id.

## 0.1.0

Logistics and fleets (FERRUM-INDUSTRY-009).

- Depots, vehicles with their capacity, and drivers with their licences.
- Trips from submitted delivery notes, a note on one open trip at a time; stops ordered nearest
  first from the depot, with a straight-line distance.
- Dispatching checks the vehicle, its load, the driver's licence on the day, and that neither is
  out on another trip; the notes carry the vehicle.
- Proof of delivery by the trip's driver, once a stop, with who took the goods and a signature or a
  photo; completing and cancelling.
- Its own interface at `/logistics/`: today's trips, and a driver's stops.
