import { describe, expect, it } from "vitest";
import { snapMove, snapResize } from "./snapping.ts";

describe("snapMove", () => {
  it("centres on the canvas when close to the middle", () => {
    const { frame, guides } = snapMove({ x: 755, y: 300, width: 400, height: 100 }, []);
    expect(frame.x).toBe(760);
    expect(guides).toContainEqual({ axis: "x", position: 960 });
  });

  it("lines up with another element's edge", () => {
    const other = { x: 100, y: 100, width: 300, height: 300 };
    const { frame } = snapMove({ x: 1000, y: 405, width: 200, height: 50 }, [other]);
    expect(frame.y).toBe(400);
  });

  it("leaves a far-away element alone", () => {
    const start = { x: 333, y: 222, width: 100, height: 100 };
    expect(snapMove(start, []).frame).toEqual(start);
  });
});

describe("snapResize", () => {
  it("snaps only the edge being dragged", () => {
    const edges = { left: false, right: true, top: false, bottom: false };
    const { frame } = snapResize({ x: 100, y: 100, width: 1815, height: 200 }, edges, []);
    expect(frame.width).toBe(1820);
    expect(frame.x).toBe(100);
  });
});
