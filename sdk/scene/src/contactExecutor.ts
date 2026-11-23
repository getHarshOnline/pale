import type { GeometryContactInput } from "./geometry";
import { SceneWorker, type WorkerLimits, type ContactResult } from "./sceneWorker";

export type { ContactResult } from "./sceneWorker";
/**
 * One off-thread Paper analysis of exact accepted geometry, without a queue.
 * Cancellation and supersession await worker termination. Failure leaves the
 * accepted geometry and explicit depth permutation usable. Byte/heap/deadline
 * ceilings have the same per-run and cumulative scope as ElkExecutor's limits,
 * not a total RSS bound.
 */
export class ContactExecutor {
  readonly #worker: SceneWorker;
  /**
   * @throws RangeError
   * If a limit is not a positive safe integer or the deadline exceeds the timer range.
   */
  constructor(limits: WorkerLimits) { this.#worker = new SceneWorker(limits); }
  /** Current single-worker lifecycle. `stopping` does not admit new work. */
  get status(): "idle" | "running" | "stopping" | "closed" { return this.#worker.status; }
  /** Cumulative wire-byte and active-time charges for this executor. */
  get charges(): Readonly<{ readonly requestBytes: number; readonly resultBytes: number; readonly activeMs: number }> {
    return this.#worker.charges;
  }
  /**
   * Analyze one accepted input. Busy/closed calls return unavailable without
   * queuing; pre-abort on an idle executor returns cancelled without starting a worker. Completion
   * means accepted estimates, not complete crossing detection.
   * Synchronous preparation/checking cannot be interrupted. Cancellation raised
   * during preparation is observed before worker construction, and overdue output
   * is rejected afterward. A started worker is terminated before the result
   * resolves, unless the result reports termination failure.
   */
  run(input: GeometryContactInput, signal?: AbortSignal): Promise<ContactResult> {
    return this.#worker.run({ kind: "contacts", input }, signal);
  }
  /**
   * Stop the current job and await release. Does not start a replacement or reopen a closed executor.
   * @throws Error
   * If termination fails; the executor remains stopping.
   */
  async supersede(): Promise<void> { await this.#worker.supersede(); }
  /**
   * Permanently close, stopping any current job. Repeated successful closes are harmless.
   * @throws Error
   * If termination fails; the executor remains stopping.
   */
  async close(): Promise<void> { await this.#worker.close(); }
}
