import type { GeometryPlan, GeometryExecutionInput } from "./geometry";
import { SceneWorker, type WorkerLimits, type LayoutResult } from "./sceneWorker";

export type { WorkerLimits as LayoutLimits, LayoutResult } from "./sceneWorker";
/**
 * One fresh ELK worker, without a queue. A stopped result follows termination.
 * Encoded-byte ceilings exclude temporary JSON allocation; heap settings exclude
 * native buffers and total RSS. The per-run deadline excludes termination;
 * cumulative active time includes it because the executor remains occupied.
 */
export class ElkExecutor {
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
   * Run one layout. Busy/closed calls return unavailable without queuing; an
   * already-aborted signal on an idle executor returns cancelled without starting a worker.
   * Synchronous preparation/checking cannot be interrupted. Cancellation raised
   * during preparation is observed before worker construction, and overdue output
   * is rejected afterward. A started worker is terminated before the result
   * resolves, unless the result reports termination failure.
   */
  run(plan: GeometryPlan | GeometryExecutionInput, signal?: AbortSignal): Promise<LayoutResult> {
    return this.#worker.run({ kind: "layout", plan }, signal);
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
