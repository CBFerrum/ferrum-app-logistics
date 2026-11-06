# Changelog

## 0.3.3

Rebuilt with Ferrum-Frontend a64416a: a new record's box whose default is 1 shows ticked, as it is saved.

## 0.3.2

- The driver's proof dialog takes a signature drawn on the screen, as well as or instead of a photo.
  It is kept on the phone with the proof, sent with it (stored once, as a PNG), and is the proof's
  Signature. A delivery or collection needs one or the other.
- From the 7 November walk: today's trips on the home link to Fleet Costs, and a trip's cost, a
  vehicle's next service day and its reading are shown formatted ("Cost 223.20", not "223.2").

## 0.3.1

Found walking it in a browser on 7 November.

- A stop's Return Note offers only return delivery notes, and its Delivery Note only those that
  are not, and each is refused on save otherwise.
- A proof saved with a connection says "Proof saved."; "A proof kept on this phone was sent."
  is said only of proofs that waited.
- Rebuilt with Ferrum-Frontend 922fcfb: an action's refusals on another record's rows show in its
  dialog, and an action with a confirmation and parameters asks once.

## 0.3.0

A driver's page that works without a connection (FERRUM-INDUSTRY-009).

- The page opens offline under the app's own worker (`/logistics/sw.js`, adapted from Point of
  Sale's), from the day's trips kept on the phone under the workspace's offline lease; what the
  lease no longer covers is forgotten at start.
- Proofs are recorded on the page itself: what happened, who took or handed over the goods, a photo
  of the goods or the signature, and a note. Each is kept on the phone for the driver alone, with
  when it happened, and sent when the server can be reached: oldest first, its photo stored once,
  its record safe to send twice. One the server refuses stays with its reason until discarded.
- Proofs say when they happened (`happened_at`): not later than now, with five minutes' grace for a
  phone's clock, nor before the trip went out. A stop's time and whether it is late go by it, on
  the workspace's clock at that moment.

## 0.2.0

Windows, collections and the fleet's running costs (FERRUM-INDUSTRY-009).

- A stop delivers or collects. A collection brings goods back with ERP's return delivery note for
  them, in draft until the goods are collected; proving the stop collected submits the note. A
  return note is one stop of one open trip.
- A stop may have a window, `HH:MM` to `HH:MM`. Ordering the stops goes by when the windows open,
  nearest first among those opening together. A stop proved after its window closed, on the
  workspace's clock, is marked late. A dispatched trip's windows may still move.
- A completed trip frees its vehicle and driver for another trip that day (tested).
- Vehicles have a cost per km: a completed trip's cost is its distance at that rate.
- Fuel logs: litres, cost and the odometer, which never goes back and moves the vehicle's reading
  on.
- Services due by the day or every so many kilometres; "It Was Serviced" records the reading and
  the next day; upkeep marks services falling due by the day.
- Fleet Costs: each vehicle's completed trips, distance and cost, and its fuel, kilometres by the
  odometer and litres per 100 km, over a range of days.
- Today's trips show their cost once completed, and the vehicles due a service. A driver's stops
  show their window and whether they deliver or collect, and link to logging fuel. Pages take the
  workspace's day, not UTC's. Built with Ferrum-Frontend a76295e.
- The drivers' endpoint defaults to the workspace's day.

## 0.1.6

Rebuilt with Ferrum-Frontend 24c3951: its pages show times on the workspace's clock, naming the zone when it is not the browser's, a draft numbered on submit is headed as a draft, and a record of an app that is not installed says so.

## 0.1.5

Rebuilt with Ferrum-Frontend bb60115: a record page reads an empty number or check as the server does, so an action whose condition needs one shows, and a workspace's settings open as their one record.

## 0.1.4

Rebuilt with Ferrum-Frontend 00dea07: in a workflow state that names what may change, the other fields show read-only, and the fields the state needs show required.

## 0.1.3

Personal-data requests find a driver by email, phone number or name; erasure blanks their name, contact details and licence, and their trips keep the anonymized driver (Ferrum's FERRUM-PRIVACY-002).

## 0.1.2

Rebuilt with Ferrum-Frontend daeadda: after an action runs, a record page shows what the action said it did rather than only that it is done.

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
