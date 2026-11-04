# Provenance

This app's design and code are its own. No code was copied from another project.

- Its shape follows common practice for delivery fleets: depots, vehicles with their capacity,
  drivers with their licences, trips of ordered stops, and proof of delivery by signature or photo.
- Distances use the haversine formula, which is public knowledge; the order is the greedy nearest
  first.
- The stops come from ERP's delivery notes, read through the platform's records.
