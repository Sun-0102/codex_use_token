import { describe, expect, it } from "vitest";
import { shouldStartCompactDrag } from "./compactDrag";

describe("shouldStartCompactDrag", () => {
  it("keeps a small pointer movement as a click", () => {
    expect(
      shouldStartCompactDrag({ x: 20, y: 20 }, { x: 22, y: 22 }),
    ).toBe(false);
  });

  it("starts native dragging after the pointer crosses the threshold", () => {
    expect(
      shouldStartCompactDrag({ x: 20, y: 20 }, { x: 26, y: 20 }),
    ).toBe(true);
  });
});
