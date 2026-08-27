import { describe, expect, it, vi } from "vitest";
import { createSingleFlight } from "./single-flight";

describe("createSingleFlight", () => {
  it("coalesces concurrent refreshes and allows a later refresh", async () => {
    const run = createSingleFlight();
    let resolveFirst!: (value: number) => void;
    const firstFactory = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          resolveFirst = resolve;
        }),
    );
    const duplicateFactory = vi.fn(async () => 2);

    const first = run(firstFactory);
    const duplicate = run(duplicateFactory);

    expect(duplicate).toBe(first);
    expect(firstFactory).toHaveBeenCalledOnce();
    expect(duplicateFactory).not.toHaveBeenCalled();

    resolveFirst(1);
    await expect(first).resolves.toBe(1);

    await expect(run(duplicateFactory)).resolves.toBe(2);
    expect(duplicateFactory).toHaveBeenCalledOnce();
  });

  it("clears a rejected request so a retry can run", async () => {
    const run = createSingleFlight();
    const failure = new Error("failed");

    await expect(run(async () => Promise.reject(failure))).rejects.toBe(failure);
    await expect(run(async () => "recovered")).resolves.toBe("recovered");
  });
});
