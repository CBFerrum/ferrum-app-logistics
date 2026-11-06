import { describe, expect, it } from "vitest";
import { proofLink, windowLabel } from "./Drive";
import { progress } from "./Home";

describe("trips and stops", () => {
  it("counts the stops done", () => {
    expect(progress({ id: "t", stops: [{ id: "a", status: "Delivered" }, { id: "b", status: "Failed" }, { id: "c", status: "Pending" }] })).toEqual({ done: 2, of: 3 });
    expect(progress({ id: "t", stops: [{ id: "a", status: "Collected" }, { id: "b" }] })).toEqual({ done: 1, of: 2 });
    expect(progress({ id: "t" })).toEqual({ done: 0, of: 0 });
  });

  it("opens a stop's proof filled in", () => {
    expect(proofLink("t1", 2, "Delivered")).toBe("/e/ProofOfDelivery/new?trip=t1&stop=2&outcome=Delivered");
    expect(proofLink("t1", 3, "Not Home")).toBe("/e/ProofOfDelivery/new?trip=t1&stop=3&outcome=Not+Home");
    expect(proofLink("t1", 4, "Collected")).toBe("/e/ProofOfDelivery/new?trip=t1&stop=4&outcome=Collected");
  });

  it("shows a stop's window", () => {
    expect(windowLabel({ id: "a", window_from: "09:00", window_to: "11:00" })).toBe("09:00–11:00");
    expect([windowLabel({ id: "a", window_from: "09:00" }), windowLabel({ id: "a", window_to: " 11:00 " }), windowLabel({ id: "a" })]).toEqual(["from 09:00", "by 11:00", ""]);
  });
});
