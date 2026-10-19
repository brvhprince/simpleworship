/**
 * Drag and drop between the library and the schedule.
 *
 * The browser's built-in drag and drop carries a small payload between
 * elements. These custom types say what is being dragged, so the schedule
 * accepts songs and its own items, and ignores anything else (a file or a
 * link dragged in from outside).
 */

/** Payload type for a library song; the data is the song id. */
export const SONG_DRAG_TYPE = "application/x-simpleworship-song";

/**
 * Payload type for media library items; the data is their ids, one per
 * line (several pictures can be dragged at once).
 */
export const MEDIA_DRAG_TYPE = "application/x-simpleworship-media";

/** Payload type for a presentation from the library; the data is its id. */
export const PRESENTATION_DRAG_TYPE = "application/x-simpleworship-presentation";

/** Reads the ids from a {@link MEDIA_DRAG_TYPE} payload. */
export function mediaIds(data: string): string[] {
  return data.split("\n").filter((id) => id.trim() !== "");
}

/** Payload type for a schedule item being reordered; the data is its id. */
export const ITEM_DRAG_TYPE = "application/x-simpleworship-schedule-item";

/**
 * Where a drop over a row would insert: before the row when the pointer is
 * in its top half, after it when in the bottom half.
 */
export function insertionIndex(rowIndex: number, pointerY: number, row: DOMRectReadOnly): number {
  const inBottomHalf = pointerY > row.top + row.height / 2;
  return inBottomHalf ? rowIndex + 1 : rowIndex;
}

/** Where a drop over a schedule row lands. */
export type DropTarget = { kind: "between"; index: number } | { kind: "into"; index: number };

/**
 * Where a drop over a row lands. When the row can take more slides and
 * what is dragged is slides (pictures, a presentation), the middle half of
 * the row adds them to it; the top and bottom quarters insert a new item
 * before or after. Otherwise the row is split in half, as for
 * {@link insertionIndex}.
 */
export function dropTarget(
  rowIndex: number,
  pointerY: number,
  row: DOMRectReadOnly,
  canDropInto: boolean,
): DropTarget {
  if (canDropInto) {
    const offset = (pointerY - row.top) / row.height;
    if (offset > 0.25 && offset < 0.75) return { kind: "into", index: rowIndex };
  }
  return { kind: "between", index: insertionIndex(rowIndex, pointerY, row) };
}

/**
 * Turns an insertion point ("between rows 2 and 3") into the final position
 * of an item moved from `from`.
 *
 * Taking the item out first shifts every later row up by one, so dropping
 * an item further *down* the list lands one place earlier than the
 * insertion point suggests.
 */
export function finalIndex(from: number, insertion: number): number {
  return insertion > from ? insertion - 1 : insertion;
}
