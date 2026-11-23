import { Buffer } from "node:buffer";
import { performance } from "node:perf_hooks";

/** Cumulative limits for one Scene compilation journey. */
export interface SceneBudgetLimits {
  readonly workUnits: number;
  /** UTF-8 bytes copied or retained by Scene-owned values. */
  readonly retainedSceneBytes: number;
  /** UTF-8 bytes in accepted renderer-neutral output. */
  readonly encodedOutputBytes: number;
  readonly deadlineMs: number;
}

/** Cumulative charges. A failed operation does not refund completed work. */
export interface SceneBudgetCharges {
  readonly workUnits: number;
  readonly retainedSceneBytes: number;
  readonly encodedOutputBytes: number;
}

/** Closed resource category reported when a Scene budget is exhausted. */
export type SceneBudgetResource = keyof SceneBudgetCharges | "deadline";

/** Invalid budget input or exhaustion of one closed resource category. */
export class SceneBudgetError extends Error {
  constructor(readonly resource: SceneBudgetResource | "limits") {
    super(`Scene budget rejected: ${resource}`);
    this.name = "SceneBudgetError";
  }
}

type BudgetState = {
  readonly limits: SceneBudgetLimits;
  readonly deadline: number;
  readonly charges: { workUnits: number; retainedSceneBytes: number; encodedOutputBytes: number };
};
const states = new WeakMap<SceneBudget, BudgetState>();

/**
 * Share one cumulative budget across intake, topology, layout, and accepted output.
 *
 * Retained byte charges cover Scene-owned copies and accepted output. They do
 * not estimate native SDK graphs, worker heaps, caller copies, or process RSS.
 * A deadline is checked between bounded operations; it cannot interrupt a
 * synchronous native call that has already started.
 */
export class SceneBudget {
  constructor(limits: SceneBudgetLimits) {
    const captured = { workUnits: limits.workUnits, retainedSceneBytes: limits.retainedSceneBytes,
      encodedOutputBytes: limits.encodedOutputBytes, deadlineMs: limits.deadlineMs };
    for (const value of Object.values(captured)) {
      if (!Number.isSafeInteger(value) || value < 1) throw new SceneBudgetError("limits");
    }
    states.set(this, { limits: Object.freeze(captured), deadline: performance.now() + captured.deadlineMs,
      charges: { workUnits: 0, retainedSceneBytes: 0, encodedOutputBytes: 0 } });
    Object.freeze(this);
  }

  /** Captured cumulative limits. Caller mutation cannot change them. */
  get limits(): SceneBudgetLimits { return states.get(this)!.limits; }
  /** Completed charges. Later failure does not refund them. */
  get charges(): SceneBudgetCharges { return Object.freeze({ ...states.get(this)!.charges }); }
}

/** Create the conservative default budget used when a caller does not narrow it. */
export function defaultSceneBudget(): SceneBudget {
  return new SceneBudget({ workUnits: 1_048_576, retainedSceneBytes: 16 * 1024 * 1024,
    encodedOutputBytes: 32 * 1024 * 1024, deadlineMs: 30_000 });
}

function state(budget: SceneBudget): BudgetState {
  const found = states.get(budget);
  if (!found) throw new TypeError("Foreign Scene budget");
  return found;
}

/** @internal Check the shared deadline before starting another bounded operation. */
export function checkSceneDeadline(budget: SceneBudget | undefined): void {
  if (budget !== undefined && performance.now() >= state(budget).deadline) throw new SceneBudgetError("deadline");
}

/** @internal Reserve performed work. The charge remains if later work fails. */
export function chargeSceneWork(budget: SceneBudget | undefined, units: number): void {
  if (budget === undefined) return;
  if (!Number.isSafeInteger(units) || units < 0) throw new SceneBudgetError("limits");
  const current = state(budget);
  checkSceneDeadline(budget);
  if (units > current.limits.workUnits - current.charges.workUnits) throw new SceneBudgetError("workUnits");
  current.charges.workUnits += units;
}

/** @internal Charge an exact number of Scene-owned retained bytes. */
export function chargeRetainedSceneBytes(budget: SceneBudget | undefined, bytes: number): void {
  if (budget === undefined) return;
  if (!Number.isSafeInteger(bytes) || bytes < 0) throw new SceneBudgetError("limits");
  const current = state(budget);
  if (bytes > current.limits.retainedSceneBytes - current.charges.retainedSceneBytes) {
    throw new SceneBudgetError("retainedSceneBytes");
  }
  current.charges.retainedSceneBytes += bytes;
  checkSceneDeadline(budget);
}

/** @internal Charge the exact UTF-8 encoding of an accepted plain value. */
export function chargeEncodedSceneOutput(budget: SceneBudget, value: unknown): number {
  checkSceneDeadline(budget);
  chargeSceneWork(budget, 1);
  const encoded = JSON.stringify(value);
  if (typeof encoded !== "string") throw new TypeError("Scene output did not encode to JSON");
  const bytes = Buffer.byteLength(encoded, "utf8"), current = state(budget);
  if (bytes > current.limits.encodedOutputBytes - current.charges.encodedOutputBytes) {
    throw new SceneBudgetError("encodedOutputBytes");
  }
  current.charges.encodedOutputBytes += bytes;
  return bytes;
}

/** @internal Count the UTF-8 encoding of a known Scene-owned plain value. */
export function retainedSceneValueBytes(value: unknown): number {
  const encoded = JSON.stringify(value);
  if (typeof encoded !== "string") throw new TypeError("Scene-owned value did not encode to JSON");
  return Buffer.byteLength(encoded, "utf8");
}
