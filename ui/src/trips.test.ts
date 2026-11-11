import { describe, expect, it } from "vitest";
import type { OfflineEnv } from "@ferrum/client";
import { outcomes, windowLabel } from "./Drive";
import { progress, TRIP_TONES } from "./Home";
import { loadTrips, missing, overlay, proofRecord, unreachable, type Proof } from "./offline";

/** A device that may keep things: a lease until next year, and storage in memory. */
function device(): OfflineEnv & { storage: Storage } {
  const m = new Map<string, string>();
  const storage = {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => void m.set(k, v),
    removeItem: (k: string) => void m.delete(k),
    key: (i: number) => [...m.keys()][i] ?? null,
    get length() {
      return m.size;
    },
    clear: () => m.clear(),
  };
  return { workspace: "fleet", lease: { allowed: true, until: "2099-01-01T00:00:00Z", hours: 72, key: "k1", person: "p1" }, storage };
}

const proof = (stop: number, extra: Partial<Proof> = {}): Proof => ({
  key: `key-${stop}`,
  trip: "t1",
  stop,
  outcome: "Delivered",
  received_by: " Bo ",
  note: "",
  photo: null,
  photo_name: "",
  happened_at: "2026-11-06T09:30:00.000Z",
  ...extra,
});

describe("trips and stops", () => {
  it("counts the stops done", () => {
    expect(progress({ id: "t", stops: [{ id: "a", status: "Delivered" }, { id: "b", status: "Failed" }, { id: "c", status: "Pending" }] })).toEqual({ done: 2, of: 3 });
    expect(progress({ id: "t", stops: [{ id: "a", status: "Collected" }, { id: "b" }] })).toEqual({ done: 1, of: 2 });
    expect(progress({ id: "t" })).toEqual({ done: 0, of: 0 });
  });

  it("shows a stop's window", () => {
    expect(windowLabel({ id: "a", window_from: "09:00", window_to: "11:00" })).toBe("09:00–11:00");
    expect([windowLabel({ id: "a", window_from: "09:00" }), windowLabel({ id: "a", window_to: " 11:00 " }), windowLabel({ id: "a" })]).toEqual(["from 09:00", "by 11:00", ""]);
  });

  it("offers a collection collected, a delivery delivered", () => {
    expect(outcomes(true)[0]).toBe("Collected");
    expect(outcomes(false)).toEqual(["Delivered", "Refused", "Damaged", "Not Home"]);
  });
});

describe("working offline", () => {
  it("asks who, and a photo or a signature, only when goods changed hands", () => {
    expect(missing("Delivered", " ", null)).toBe("Name who took the goods.");
    expect(missing("Collected", "", null)).toBe("Name who handed the goods over.");
    expect(missing("Delivered", "Bo", null)).toBe("Take a photo of the goods, or have them sign on the screen.");
    expect(missing("Delivered", "Bo", new Blob(["x"]))).toBeNull();
    expect(missing("Collected", "Ivan", null, new Blob(["signed"]))).toBeNull();
    expect(missing("Not Home", "", null)).toBeNull();
  });

  it("sends a proof with when it happened and its stored photo or signature", () => {
    expect(proofRecord(proof(2, { photo_url: "/files/door.jpg", note: "  " }))).toEqual({
      trip: "t1",
      stop: 2,
      outcome: "Delivered",
      received_by: "Bo",
      note: null,
      photo: "/files/door.jpg",
      signature: null,
      happened_at: "2026-11-06T09:30:00.000Z",
    });
    expect(proofRecord(proof(3, { signature_url: "/files/signature.png" }))).toMatchObject({ photo: null, signature: "/files/signature.png" });
  });

  it("shows a kept proof on its pending stop only", () => {
    const trips = [
      { id: "t1", stops: [{ id: "a", sequence: 1, status: "Pending" }, { id: "b", sequence: 2, status: "Delivered" }] },
      { id: "t2", stops: [{ id: "c", sequence: 1, status: "Pending" }] },
    ];
    const [one, two] = overlay(trips, [proof(1), proof(2)]);
    const stops = (t: typeof one) => t.stops as { kept?: Proof }[];
    expect(stops(one).map((s) => s.kept?.key)).toEqual(["key-1", undefined]);
    expect(stops(two)[0].kept).toBeUndefined();
  });

  it("tells a lost connection from a refusal", () => {
    expect([new TypeError("Failed to fetch"), new DOMException("late", "TimeoutError"), { status: 0 }, { status: 401 }, { status: 503 }].every(unreachable)).toBe(true);
    expect([{ status: 403 }, { status: 422 }, new Error("no")].some(unreachable)).toBe(false);
  });

  it("opens the day's trips from this phone when the server cannot be reached", async () => {
    const env = device();
    const day = "2026-11-10";
    const trips = [{ id: "t1", number: "TRP-1", stops: [] }];
    expect(await loadTrips(day, env, () => Promise.resolve({ trips }))).toEqual({ trips, asOf: null });
    const offline = await loadTrips(day, env, () => Promise.reject(new TypeError("Failed to fetch")));
    expect(offline.trips).toEqual(trips);
    expect(offline.asOf).not.toBeNull();
    // Not another day's, and not when the server refuses.
    await expect(loadTrips("2026-11-11", env, () => Promise.reject(new TypeError("Failed to fetch")))).rejects.toThrow("Failed to fetch");
    await expect(loadTrips(day, env, () => Promise.reject(Object.assign(new Error("no"), { status: 403 })))).rejects.toThrow("no");
    // Nothing kept once the lease has ended: the store holds the lease alone.
    const ended = { ...device(), lease: { allowed: true, until: "2020-01-01T00:00:00Z", hours: 72, key: "k1", person: "p1" } };
    await loadTrips(day, ended, () => Promise.resolve({ trips }));
    expect(ended.storage.length).toBe(1);
  });
});

describe("the trips' badges", () => {
  it("takes a trip's tone from its status", () => {
    expect(TRIP_TONES).toEqual({ Planned: "neutral", Dispatched: "blue", Completed: "green", Cancelled: "neutral" });
  });
});
