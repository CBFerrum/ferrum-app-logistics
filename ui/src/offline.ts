// A driver on the road without a connection (Ferrum docs/offline.md). The day's trips are kept on
// this phone under the workspace's lease, so the driver's page opens offline. A proof recorded
// offline is kept for the driver alone, photo and signature and all, with the time it happened, until it is
// sent: oldest first, each safe to send again (its key), stopping at the first the server cannot
// be reached for. One the server refuses stays, with its reason, until the driver discards it.

import { api, ApiError, cacheName, docPath, errorMessage, offlineEnv, pendingName, uploadFile, type OfflineEnv } from "@ferrum/client";
import type { Row } from "./Home";

/** How long the page waits for the server before it shows what this phone kept. */
const PATIENCE_MS = 5000;

/** Whether an error means the server could not be reached, or the sign-in lapsed: what was done is
 *  kept and sent later. */
export function unreachable(e: unknown): boolean {
  if (e instanceof TypeError || (e instanceof DOMException && (e.name === "TimeoutError" || e.name === "AbortError"))) return true;
  const status = (e as { status?: number } | null)?.status;
  return status === 0 || status === 401 || status === 408 || status === 429 || (typeof status === "number" && status >= 500);
}

// ── the day's trips ──

interface KeptTrips {
  day: string;
  at: string;
  trips: Row[];
}

const fetchTrips = (day: string) => api.get<{ trips: Row[] }>("/api/app/logistics/drive", { day }, AbortSignal.timeout(PATIENCE_MS));

/** The driver's trips of `day`, from the server, kept for later; when it cannot be reached, the
 *  ones kept, with when they were (`asOf`). */
export async function loadTrips(day: string, env: OfflineEnv = offlineEnv(), get = fetchTrips): Promise<{ trips: Row[]; asOf: string | null }> {
  const name = cacheName(env, "logistics", "drive");
  try {
    const r = await get(day);
    try {
      if (name) env.storage?.setItem(name, JSON.stringify({ day, at: new Date().toISOString(), trips: r.trips } satisfies KeptTrips));
    } catch {
      // A full or refused store: the page works online only.
    }
    return { trips: r.trips, asOf: null };
  } catch (e) {
    let kept: KeptTrips | null = null;
    try {
      kept = name && unreachable(e) ? (JSON.parse(env.storage?.getItem(name) ?? "null") as KeptTrips | null) : null;
    } catch {
      kept = null;
    }
    if (kept?.day !== day) throw e;
    return { trips: kept.trips, asOf: kept.at };
  }
}

// ── proofs not sent yet ──

export interface Proof {
  /** Its idempotency key: sent twice, stored once. */
  key: string;
  trip: string;
  stop: number;
  outcome: string;
  received_by: string;
  note: string;
  photo: Blob | null;
  photo_name: string;
  /** The photo's address once stored, so a proof sent again does not store it twice. */
  photo_url?: string;
  /** A signature drawn on the screen, as a PNG (none on proofs kept before 0.3.2). */
  signature?: Blob | null;
  signature_url?: string;
  happened_at: string;
  /** Why the server would not take it. */
  refused?: string;
}

/** What a proof still needs before it is kept: who, and a photo or a signature, when goods changed
 *  hands. */
export function missing(outcome: string, receivedBy: string, photo: Blob | null, signature: Blob | null = null): string | null {
  if (outcome !== "Delivered" && outcome !== "Collected") return null;
  if (!receivedBy.trim()) return outcome === "Collected" ? "Name who handed the goods over." : "Name who took the goods.";
  return photo || signature ? null : "Take a photo of the goods, or have them sign on the screen.";
}

/** The proof's record, its photo and signature stored. */
export function proofRecord(p: Proof): Record<string, unknown> {
  return {
    trip: p.trip,
    stop: p.stop,
    outcome: p.outcome,
    received_by: p.received_by.trim() || null,
    note: p.note.trim() || null,
    photo: p.photo_url ?? null,
    signature: p.signature_url ?? null,
    happened_at: p.happened_at,
  };
}

/** The driver's trips with the proofs kept on this phone on their stops (`kept`), still pending
 *  as far as the server knows. */
export function overlay(trips: Row[], kept: Proof[]): Row[] {
  return trips.map((t) => ({
    ...t,
    stops: (Array.isArray(t.stops) ? (t.stops as Row[]) : []).map((s) => {
      const p = kept.find((k) => k.trip === t.id && k.stop === Number(s.sequence));
      return p && (s.status ?? "Pending") === "Pending" ? { ...s, kept: p } : s;
    }),
  }));
}

const STORE = "proofs";

/** The driver's own store of proofs on this phone; null where nothing can be kept. */
function openStore(env: OfflineEnv): Promise<IDBDatabase> | null {
  const name = pendingName(env, "logistics", STORE);
  if (!name || typeof indexedDB === "undefined") return null;
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(name, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE, { keyPath: "key" });
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("This phone keeps nothing offline."));
  });
}

async function inStore<T>(mode: IDBTransactionMode, run: (s: IDBObjectStore) => IDBRequest, env = offlineEnv()): Promise<T> {
  const open = openStore(env);
  if (!open) throw new Error("This phone keeps nothing offline: send proofs with a connection.");
  const db = await open;
  return new Promise<T>((resolve, reject) => {
    const tx = db.transaction(STORE, mode);
    const req = run(tx.objectStore(STORE));
    tx.oncomplete = () => (db.close(), resolve(req.result as T));
    tx.onerror = tx.onabort = () => (db.close(), reject(tx.error ?? new Error("This phone could not keep it.")));
  });
}

export const keepProof = (p: Proof) => inStore<unknown>("readwrite", (s) => s.put(p)).then(() => undefined);
export const forgetProof = (key: string) => inStore<unknown>("readwrite", (s) => s.delete(key)).then(() => undefined);
/** The proofs kept on this phone, oldest first; none where nothing can be kept. */
export const keptProofs = () =>
  inStore<Proof[]>("readonly", (s) => s.getAll()).then(
    (l) => l.sort((a, b) => a.happened_at.localeCompare(b.happened_at)),
    () => [],
  );

/** Sends a proof: its photo and signature stored first (once each), then its record, safe to send
 *  again. */
export async function sendProof(p: Proof): Promise<void> {
  if (p.photo && !p.photo_url) {
    const file = new File([p.photo], p.photo_name || "proof.jpg", { type: p.photo.type || "image/jpeg" });
    p.photo_url = (await uploadFile(file)).url;
    await keepProof(p).catch(() => {});
  }
  if (p.signature && !p.signature_url) {
    p.signature_url = (await uploadFile(new File([p.signature], "signature.png", { type: "image/png" }))).url;
    await keepProof(p).catch(() => {});
  }
  await api.post(docPath("ProofOfDelivery"), proofRecord(p), { idempotencyKey: p.key });
}

let flushing: Promise<number> | null = null;

/** Sends the proofs kept on this phone, oldest first, one run at a time: how many went. */
export function flush(): Promise<number> {
  flushing ??= (async () => {
    let sent = 0;
    for (const p of await keptProofs()) {
      if (p.refused) continue;
      try {
        await sendProof(p);
        await forgetProof(p.key);
        sent++;
      } catch (e) {
        // Unreachable, or this page is older than the app: kept, to go later.
        if (unreachable(e) || (e instanceof ApiError && e.code === "ui_outdated")) break;
        await keepProof({ ...p, refused: errorMessage(e) });
      }
    }
    return sent;
  })().finally(() => {
    flushing = null;
  });
  return flushing;
}
