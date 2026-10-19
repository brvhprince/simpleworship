/**
 * Snap guides for the canvas editor: while an element is dragged, its
 * edges and centre jump to the canvas's edges and centre lines and to
 * other elements' edges and centres when they come close, and a guide
 * line shows what it lined up with.
 */

import type { Frame } from "@simpleworship/protocol";
import { CANVAS_HEIGHT, CANVAS_WIDTH } from "@simpleworship/renderer";

/** How close (in canvas pixels) an edge must come to snap. */
export const SNAP_DISTANCE = 12;

/** A guide line: vertical at `x`, or horizontal at `y`. */
export interface Guide {
  axis: "x" | "y";
  position: number;
}

/** The lines an element can line up with, on each axis. */
export function snapLines(others: Frame[]): { x: number[]; y: number[] } {
  const x = [0, CANVAS_WIDTH / 2, CANVAS_WIDTH];
  const y = [0, CANVAS_HEIGHT / 2, CANVAS_HEIGHT];
  for (const frame of others) {
    x.push(frame.x, frame.x + frame.width / 2, frame.x + frame.width);
    y.push(frame.y, frame.y + frame.height / 2, frame.y + frame.height);
  }
  return { x, y };
}

/** The smallest shift (within `SNAP_DISTANCE`) that puts one of `points` on a line. */
function nearest(points: number[], lines: number[]): { shift: number; line: number } | null {
  let best: { shift: number; line: number } | null = null;
  for (const point of points) {
    for (const line of lines) {
      const shift = line - point;
      if (Math.abs(shift) <= SNAP_DISTANCE && (!best || Math.abs(shift) < Math.abs(best.shift))) {
        best = { shift, line };
      }
    }
  }
  return best;
}

/** A moved frame, snapped by its edges and centre, and the guides to show. */
export function snapMove(frame: Frame, others: Frame[]): { frame: Frame; guides: Guide[] } {
  const lines = snapLines(others);
  const guides: Guide[] = [];
  const snapped = { ...frame };
  const x = nearest([frame.x, frame.x + frame.width / 2, frame.x + frame.width], lines.x);
  if (x) {
    snapped.x += x.shift;
    guides.push({ axis: "x", position: x.line });
  }
  const y = nearest([frame.y, frame.y + frame.height / 2, frame.y + frame.height], lines.y);
  if (y) {
    snapped.y += y.shift;
    guides.push({ axis: "y", position: y.line });
  }
  return { frame: snapped, guides };
}

/**
 * A resized frame, snapped by the edges being dragged (`left`/`right`,
 * `top`/`bottom`), and the guides to show.
 */
export function snapResize(
  frame: Frame,
  edges: { left: boolean; right: boolean; top: boolean; bottom: boolean },
  others: Frame[],
): { frame: Frame; guides: Guide[] } {
  const lines = snapLines(others);
  const guides: Guide[] = [];
  const snapped = { ...frame };
  if (edges.left || edges.right) {
    const edge = edges.left ? frame.x : frame.x + frame.width;
    const hit = nearest([edge], lines.x);
    if (hit) {
      if (edges.left) {
        snapped.x += hit.shift;
        snapped.width -= hit.shift;
      } else {
        snapped.width += hit.shift;
      }
      guides.push({ axis: "x", position: hit.line });
    }
  }
  if (edges.top || edges.bottom) {
    const edge = edges.top ? frame.y : frame.y + frame.height;
    const hit = nearest([edge], lines.y);
    if (hit) {
      if (edges.top) {
        snapped.y += hit.shift;
        snapped.height -= hit.shift;
      } else {
        snapped.height += hit.shift;
      }
      guides.push({ axis: "y", position: hit.line });
    }
  }
  return { frame: snapped, guides };
}
