//! Distances as the crow flies (the haversine formula on a sphere of the Earth's mean radius), and
//! stops ordered nearest first. ponytail: straight lines and a greedy order; road distances and an
//! optimised order need a routing service (BLOCK-MAPS-KEY).

/// A point: degrees north and east.
pub(crate) type Point = (f64, f64);

/// The great-circle distance between two points, in kilometres.
pub(crate) fn km(a: Point, b: Point) -> f64 {
    const R: f64 = 6371.0088;
    let (la1, la2) = (a.0.to_radians(), b.0.to_radians());
    let (dla, dlo) = ((b.0 - a.0).to_radians(), (b.1 - a.1).to_radians());
    let h = (dla / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlo / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().asin()
}

/// The order to visit `stops` in from `start`: each time the nearest not yet visited.
pub(crate) fn nearest_first(start: Point, stops: &[Point]) -> Vec<usize> {
    let mut left: Vec<usize> = (0..stops.len()).collect();
    let mut at = start;
    let mut out = Vec::with_capacity(stops.len());
    while !left.is_empty() {
        let (i, _) = left.iter().enumerate().map(|(i, s)| (i, km(at, stops[*s]))).min_by(|a, b| a.1.total_cmp(&b.1)).expect("one left");
        let next = left.remove(i);
        at = stops[next];
        out.push(next);
    }
    out
}

/// From `start` through `stops` in order and back, in kilometres.
pub(crate) fn round_trip(start: Point, stops: &[Point]) -> f64 {
    let mut at = start;
    let mut total = 0.0;
    for s in stops {
        total += km(at, *s);
        at = *s;
    }
    total + km(at, start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_and_the_nearest_first() {
        // London to Paris is about 344 km as the crow flies.
        let (london, paris) = ((51.5074, -0.1278), (48.8566, 2.3522));
        assert!((km(london, paris) - 343.5).abs() < 2.0, "{}", km(london, paris));
        assert_eq!(km(london, london), 0.0);
        let depot = (0.0, 0.0);
        let far = (0.0, 3.0);
        let near = (0.0, 1.0);
        let mid = (0.0, 2.0);
        assert_eq!(nearest_first(depot, &[far, near, mid]), vec![1, 2, 0]);
        let there_and_back = round_trip(depot, &[near, mid, far]);
        assert!((there_and_back - 2.0 * km(depot, far)).abs() < 1e-6);
    }
}
