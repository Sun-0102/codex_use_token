export interface PointerPosition {
  x: number;
  y: number;
}

const COMPACT_DRAG_THRESHOLD_PX = 5;

export function shouldStartCompactDrag(
  origin: PointerPosition,
  current: PointerPosition,
): boolean {
  return (
    Math.hypot(current.x - origin.x, current.y - origin.y) >=
    COMPACT_DRAG_THRESHOLD_PX
  );
}
