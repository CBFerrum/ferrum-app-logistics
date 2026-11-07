// A driver's page: today's dispatched trips that are theirs (from /api/app/logistics/drive, which
// finds them by the address they signed in with), each stop in order with its address, whether it
// delivers or collects and its window, and its proof recorded here, with a photo or a signature
// drawn on the screen. It works without a
// connection: the trips are kept on the phone, and a proof is kept there with when it happened
// until it is sent (./offline). Fuel is logged with a connection.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState, type PointerEvent as Pointer } from "react";
import { Link } from "react-router";
import { errorMessage, formatMoment, newKey, todayIso, t as tr, isolate } from "@ferrum/client";
import { Badge, Button, Dialog, EmptyState, ErrorState, Field, PageLoading, statusTone, toast, useShellTitle } from "@ferrum/kit";
import type { Row } from "./Home";
import { flush, forgetProof, keepProof, keptProofs, loadTrips, missing, overlay, sendProof, unreachable, type Proof } from "./offline";

/** A stop's window, as "09:00–11:00", "from 09:00" or "by 11:00"; empty without one. */
export function windowLabel(stop: Row): string {
  const [from, to] = [stop.window_from, stop.window_to].map((v) => (typeof v === "string" ? v.trim() : ""));
  if (from && to) return `${from}–${to}`;
  return from ? tr("from {from}", { from }) : to ? tr("by {to}", { to }) : "";
}

/** What a stop may be proved with: delivered or collected, as it is, or why not. */
export function outcomes(collect: boolean): string[] {
  return [collect ? "Collected" : "Delivered", "Refused", "Damaged", "Not Home"];
}

export function Drive() {
  useShellTitle(tr("Your stops"));
  const day = todayIso();
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["logistics", "drive", day], queryFn: () => loadTrips(day), refetchInterval: 60_000 });
  const kept = useQuery({ queryKey: ["logistics", "kept"], queryFn: keptProofs });
  const [proving, setProving] = useState<{ trip: Row; stop: Row; outcome: string } | null>(null);
  // `quiet` right after a proof is saved: it was said already, and the waiting line shows what is left.
  const sendKept = useCallback(
    (quiet = false) => {
      void flush()
        .then((n) => {
          if (n && !quiet) toast(n === 1 ? tr("A proof kept on this phone was sent.") : tr("{n} proofs kept on this phone were sent.", { n }));
        })
        .catch(() => {})
        .finally(() => void qc.invalidateQueries({ queryKey: ["logistics"] }));
    },
    [qc],
  );
  useEffect(() => {
    const go = () => sendKept();
    go();
    window.addEventListener("online", go);
    const every = setInterval(go, 60_000);
    return () => {
      window.removeEventListener("online", go);
      clearInterval(every);
    };
  }, [sendKept]);

  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const proofs = kept.data ?? [];
  const waiting = proofs.filter((p) => !p.refused).length;
  const discard = (key: string) => void forgetProof(key).finally(() => void qc.invalidateQueries({ queryKey: ["logistics", "kept"] }));
  return (
    <div className="np-page stack">
      {q.data.asOf && <p role="status" className="faint">{tr("No connection: your trips as of {value}.", { value: formatMoment(q.data.asOf, { timeStyle: "short" }) })}</p>}
      {waiting > 0 && (
        <p role="status" className="row">
          <span>{waiting === 1 ? tr("1 proof is kept on this phone, to send.") : tr("{waiting} proofs are kept on this phone, to send.", { waiting })}</span>
          <Button size="sm" onClick={() => sendKept()}>
            {tr("Send now")}
          </Button>
        </p>
      )}
      {proofs
        .filter((p) => p.refused)
        .map((p) => (
          <p key={p.key} role="alert" className="row">
            <span>{tr("Stop {stop}'s proof was not taken: {refused}", { stop: p.stop, refused: String(p.refused) })}</span>
            <Button size="sm" variant="ghost" onClick={() => discard(p.key)}>
              {tr("Discard")}
            </Button>
          </p>
        ))}
      <span className="row">
        <Link to="/e/FuelLog/new">{tr("Log fuel")}</Link>
      </span>
      {!q.data.trips.length && <EmptyState title={tr("No trip out for you today")} icon="truck" />}
      {overlay(q.data.trips, proofs).map((t) => (
        <section key={t.id} className="card card-pad stack" aria-label={String(t.number ?? "")}>
          <h2>{isolate(String(t.number ?? ""))}</h2>
          <ol className="stack">
            {(t.stops as Row[]).map((s) => {
              const collect = s.kind === "Collection";
              const slot = windowLabel(s);
              const mine = s.kept as Proof | undefined;
              const status = mine ? (mine.refused ? "Not taken" : "Kept, to send") : String(s.status ?? "Pending");
              const open = (outcome: string) => setProving({ trip: t, stop: s, outcome });
              return (
                <li key={String(s.id ?? s.sequence)} className="stack">
                  <span>
                    <strong>{isolate(String(s.sequence))}.</strong> {collect ? tr("Collect from ") : ""}
                    {tr("{value} · {value2} kg", { value: String(s.address ?? ""), value2: String(s.weight_kg ?? 0) })}{slot ? ` · ${slot}` : ""}{isolate(" ")}
                    <Badge tone={mine ? "amber" : statusTone(status)}>{isolate(status)}</Badge>
                    {s.late === true && <Badge tone="amber">{tr("Late")}</Badge>}
                  </span>
                  {!mine && (s.status ?? "Pending") === "Pending" && (
                    <span className="row">
                      <Button size="sm" variant="primary" onClick={() => open(collect ? "Collected" : "Delivered")}>
                        {collect ? tr("Collected") : tr("Delivered")}
                      </Button>
                      <Button size="sm" onClick={() => open("Not Home")}>
                        {collect ? tr("Not collected") : tr("Not delivered")}
                      </Button>
                    </span>
                  )}
                </li>
              );
            })}
          </ol>
        </section>
      ))}
      {proving && (
        <ProofDialog
          {...proving}
          onClose={() => setProving(null)}
          onDone={() => {
            setProving(null);
            toast(tr("Proof saved."));
            sendKept(true);
          }}
        />
      )}
    </div>
  );
}

/** A stop's proof, kept on the phone with the time it happened, then sent; sent at once where the
 *  phone keeps nothing. */
function ProofDialog({ trip, stop, outcome: first, onClose, onDone }: { trip: Row; stop: Row; outcome: string; onClose: () => void; onDone: () => void }) {
  const collect = stop.kind === "Collection";
  const [outcome, setOutcome] = useState(first);
  const [receivedBy, setReceivedBy] = useState("");
  const [note, setNote] = useState("");
  const [photo, setPhoto] = useState<File | null>(null);
  const [signature, setSignature] = useState<Blob | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const handed = outcome === "Delivered" || outcome === "Collected";
  const save = async () => {
    const why = missing(outcome, receivedBy, photo, signature);
    if (why) return setError(why);
    const p: Proof = {
      key: newKey(),
      trip: trip.id,
      stop: Number(stop.sequence),
      outcome,
      received_by: handed ? receivedBy : "",
      note,
      photo: handed ? photo : null,
      photo_name: handed ? (photo?.name ?? "") : "",
      signature: handed ? signature : null,
      happened_at: new Date().toISOString(),
    };
    setBusy(true);
    try {
      await keepProof(p).catch(() => sendProof(p));
      onDone();
    } catch (e) {
      setError(unreachable(e) ? "No connection, and this phone keeps nothing offline: try again with a connection." : errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onClose={onClose}
      title={tr("Stop {value}: {value2}", { value: String(stop.sequence), value2: String(stop.address ?? "") })}
      footer={
        <>
          <Button onClick={onClose}>{tr("Cancel")}</Button>
          <Button variant="primary" loading={busy} onClick={() => void save()}>
            {tr("Save")}
          </Button>
        </>
      }
    >
      <div className="stack">
        <Field label={tr("What happened")} required>
          <select className="select" value={outcome} onChange={(e) => setOutcome(e.target.value)}>
            {outcomes(collect).map((o) => (
              <option key={o} value={o}>
                {isolate(o)}
              </option>
            ))}
          </select>
        </Field>
        {handed && (
          <Field label={collect ? tr("Handed over by") : tr("Received by")} required>
            <input className="input" value={receivedBy} autoComplete="off" onChange={(e) => setReceivedBy(e.target.value)} />
          </Field>
        )}
        {handed && (
          <Field label={tr("Photo of the goods or the signature")}>
            <input className="input" type="file" accept="image/*" capture="environment" onChange={(e) => setPhoto(e.target.files?.[0] ?? null)} />
          </Field>
        )}
        {handed && (
          <Field label={tr("Or they sign here")}>
            <SignaturePad onChange={setSignature} />
          </Field>
        )}
        <Field label={tr("Note")}>
          <textarea className="textarea" rows={2} value={note} onChange={(e) => setNote(e.target.value)} />
        </Field>
        {error && (
          <p className="field-error" role="alert">
            {isolate(error)}
          </p>
        )}
      </div>
    </Dialog>
  );
}

/** A signature drawn on the screen with a finger or a pen, kept as a PNG on white; none until a
 *  line is drawn, and none again once cleared. */
function SignaturePad({ onChange }: { onChange: (png: Blob | null) => void }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const last = useRef<{ x: number; y: number } | null>(null);
  const drawn = useRef(false);
  const blank = () => {
    const c = ref.current;
    const g = c?.getContext("2d");
    if (!c || !g) return;
    g.fillStyle = "#fff";
    g.fillRect(0, 0, c.width, c.height);
    drawn.current = false;
  };
  useEffect(blank, []);
  // The canvas keeps its own pixels whatever size it is shown at.
  const at = (e: Pointer<HTMLCanvasElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    return { x: ((e.clientX - r.left) * e.currentTarget.width) / r.width, y: ((e.clientY - r.top) * e.currentTarget.height) / r.height };
  };
  const draw = (e: Pointer<HTMLCanvasElement>) => {
    const from = last.current;
    const g = e.currentTarget.getContext("2d");
    if (!from || !g) return;
    const to = at(e);
    g.strokeStyle = "#111";
    g.lineWidth = 4;
    g.lineCap = "round";
    g.beginPath();
    g.moveTo(from.x, from.y);
    g.lineTo(to.x, to.y);
    g.stroke();
    last.current = to;
    drawn.current = true;
  };
  const end = (e: Pointer<HTMLCanvasElement>) => {
    if (!last.current) return;
    last.current = null;
    if (drawn.current) e.currentTarget.toBlob((png) => onChange(png), "image/png");
  };
  return (
    <div className="stack">
      <canvas
        ref={ref}
        width={600}
        height={200}
        role="img"
        aria-label={tr("Signature: draw it here")}
        style={{ width: "100%", height: 140, touchAction: "none", borderRadius: 6, border: "1px solid var(--border, #ccc)" }}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          last.current = at(e);
        }}
        onPointerMove={draw}
        onPointerUp={end}
        onPointerCancel={end}
      />
      <Button
        size="sm"
        onClick={() => {
          blank();
          onChange(null);
        }}
      >
        {tr("Clear the signature")}
      </Button>
    </div>
  );
}
