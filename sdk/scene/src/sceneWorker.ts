import { Buffer } from "node:buffer";
import { join } from "node:path";
import { performance } from "node:perf_hooks";
import { Worker } from "node:worker_threads";
import { GeometryPlan, GeometryExecutionInput, GeometryContactInput, type GeometryResult } from "./geometry";
import type { ContactAnalysis } from "./contactAnalysis";

const executionInput = GeometryExecutionInput.prototype.elkInput;
const executionAccept = GeometryExecutionInput.prototype.accept;
const contactInput = GeometryContactInput.prototype.workerInput;
const contactAccept = GeometryContactInput.prototype.accept;

/**
 * Limits shared by the layout and contact executors, captured at construction.
 * All values are positive safe integers. Encoded bytes exclude temporary JSON
 * allocations; Node's worker-heap settings exclude native buffers and total RSS.
 */
export interface WorkerLimits {
  /** Maximum UTF-8 bytes of the serialized request. */
  readonly requestBytes: number;
  /** Maximum UTF-8 bytes of the serialized response. */
  readonly resultBytes: number;
  /** Milliseconds for preparation, worker work and acceptance; excludes termination. Maximum 2,147,483,647. */
  readonly deadlineMs: number;
  /** Maximum cumulative request bytes admitted during this executor's lifetime. */
  readonly totalRequestBytes: number;
  /** Maximum cumulative response bytes received during this executor's lifetime. */
  readonly totalResultBytes: number;
  /** Maximum cumulative milliseconds from accepted start through worker release; idle time is excluded. */
  readonly totalActiveMs: number;
  /** Node worker old-generation heap setting, in MB. */
  readonly oldHeapMb: number;
  /** Node worker young-generation heap setting, in MB. */
  readonly youngHeapMb: number;
}
/** A completed layout is accepted geometry. Termination failure leaves the executor stopping. */
export type LayoutResult =
  | { readonly kind: "completed"; readonly geometry: GeometryResult }
  | { readonly kind: "unavailable"; readonly reason: "busy" | "closed" }
  | { readonly kind: "stopped"; readonly reason: "cancelled" | "superseded" | "shutdown" | "deadline" }
  | { readonly kind: "failed"; readonly reason: "request-bytes" | "result-bytes" | "worker" | "geometry" | "termination" };
/**
 * Completed means estimates were accepted, not that every contact was found.
 * Numeric/load/bounds refusals and transport failures leave the input geometry usable.
 * Analysis denotes request/response acceptance failure; termination failure leaves
 * the executor stopping, with resource release unconfirmed.
 */
export type ContactResult = Exclude<LayoutResult, { readonly kind: "completed" | "failed" }>
  | { readonly kind: "completed"; readonly analysis: ContactAnalysis }
  | { readonly kind: "failed"; readonly reason: "request-bytes" | "result-bytes" | "worker" | "analysis" | "numeric" | "bounds" | "load" | "termination" };
type WorkResult = LayoutResult | ContactResult;
type Request = { readonly kind: "layout"; readonly plan: GeometryPlan | GeometryExecutionInput }
  | { readonly kind: "contacts"; readonly input: GeometryContactInput };
type Preparation = {
  readonly result: Promise<WorkResult>;
  readonly resolve: (result: WorkResult) => void;
  readonly started: number;
  destination: "idle" | "closed";
  outcome: WorkResult | undefined;
};
type Job = Preparation & {
  readonly worker: Worker;
  readonly signal: AbortSignal | undefined;
  readonly onAbort: () => void;
  timer: ReturnType<typeof setTimeout> | undefined;
};
type State = { readonly kind: "idle" } | { readonly kind: "closed" }
  | { readonly kind: "preparing"; readonly preparation: Preparation }
  | { readonly kind: "running"; readonly job: Job } | { readonly kind: "stopping"; readonly job: Job };

/** @internal
 * Private, closed layout/contact lifecycle. One fresh worker, without a queue.
 * Per-run and cumulative message limits bound encoded bytes, not temporary JSON
 * allocations. With one worker at a time, heap limits cap each active worker;
 * they do not bound native buffers, retained caller results or total process memory.
 * Per-run time covers preparation, work and checking. Cumulative active time also
 * covers termination because the executor cannot admit replacement work before release.
 * Synchronous work cannot be interrupted; an overdue result is rejected afterward.
 * A stopped result means termination finished, not merely that abort was requested.
 */
export class SceneWorker {
  #state: State = { kind: "idle" };
  readonly #limits: WorkerLimits;
  readonly #used = { requestBytes: 0, resultBytes: 0, activeMs: 0 };

  constructor(limits: WorkerLimits) {
    limits = { requestBytes: limits.requestBytes, resultBytes: limits.resultBytes,
      deadlineMs: limits.deadlineMs, totalRequestBytes: limits.totalRequestBytes,
      totalResultBytes: limits.totalResultBytes, totalActiveMs: limits.totalActiveMs,
      oldHeapMb: limits.oldHeapMb, youngHeapMb: limits.youngHeapMb };
    for (const value of [limits.requestBytes, limits.resultBytes, limits.deadlineMs,
      limits.totalRequestBytes, limits.totalResultBytes, limits.totalActiveMs,
      limits.oldHeapMb, limits.youngHeapMb]) {
      if (!Number.isSafeInteger(value) || value < 1) throw new RangeError("Scene worker limits must be positive safe integers");
    }
    if (limits.deadlineMs > 2_147_483_647) throw new RangeError("Scene worker deadline exceeds the timer range");
    this.#limits = limits;
  }

  /** No geometry is accepted while stopping, including a completion already in transit. */
  get status(): "idle" | "running" | "stopping" | "closed" {
    return this.#state.kind === "preparing" ? "running" : this.#state.kind;
  }
  /** Cumulative admitted wire bytes and active time. Returned values cannot mutate the executor. */
  get charges(): Readonly<{ readonly requestBytes: number; readonly resultBytes: number; readonly activeMs: number }> {
    return Object.freeze({ ...this.#used });
  }

  run(request: Extract<Request, { readonly kind: "layout" }>, signal?: AbortSignal): Promise<LayoutResult>;
  run(request: Extract<Request, { readonly kind: "contacts" }>, signal?: AbortSignal): Promise<ContactResult>;
  run(request: Request, signal?: AbortSignal): Promise<WorkResult> {
    if (this.#state.kind === "closed") return Promise.resolve({ kind: "unavailable", reason: "closed" });
    if (this.#state.kind !== "idle") return Promise.resolve({ kind: "unavailable", reason: "busy" });
    if (signal?.aborted) return Promise.resolve({ kind: "stopped", reason: "cancelled" });
    const started = performance.now();
    const activeRemaining = this.#limits.totalActiveMs - this.#used.activeMs;
    if (activeRemaining <= 0) return Promise.resolve({ kind: "stopped", reason: "deadline" });
    const deadline = started + Math.min(this.#limits.deadlineMs, activeRemaining);
    let resolve!: (result: WorkResult) => void;
    const result = new Promise<WorkResult>(complete => { resolve = complete; });
    const preparation: Preparation = { result, resolve, started, destination: "idle", outcome: undefined };
    this.#state = { kind: "preparing", preparation };
    let input: string;
    try {
      const encoded = JSON.stringify(request.kind === "contacts" ? contactInput.call(request.input)
        : request.plan instanceof GeometryExecutionInput ? executionInput.call(request.plan) : request.plan.elkInput());
      if (typeof encoded !== "string") throw new TypeError("Scene worker request did not encode to JSON");
      input = encoded;
    } catch {
      return this.#finishPreparation(preparation,
        { kind: "failed", reason: request.kind === "contacts" ? "analysis" : "geometry" });
    }
    const inputBytes = Buffer.byteLength(input, "utf8");
    if (inputBytes > this.#limits.requestBytes
      || inputBytes > this.#limits.totalRequestBytes - this.#used.requestBytes) {
      return this.#finishPreparation(preparation, { kind: "failed", reason: "request-bytes" });
    }
    this.#used.requestBytes += inputBytes;
    if (!preparation.outcome && signal?.aborted) preparation.outcome = { kind: "stopped", reason: "cancelled" };
    if (!preparation.outcome && performance.now() >= deadline) preparation.outcome = { kind: "stopped", reason: "deadline" };
    if (preparation.outcome) return this.#finishPreparation(preparation, preparation.outcome);
    const resultRemaining = this.#limits.totalResultBytes - this.#used.resultBytes;
    if (resultRemaining < 1) {
      return this.#finishPreparation(preparation, { kind: "failed", reason: "result-bytes" });
    }
    let worker: Worker;
    try {
      worker = new Worker(join(__dirname, request.kind === "layout" ? "elkWorker.js" : "paperWorker.js"), {
        workerData: { input, resultBytes: Math.min(this.#limits.resultBytes, resultRemaining) },
        resourceLimits: { maxOldGenerationSizeMb: this.#limits.oldHeapMb, maxYoungGenerationSizeMb: this.#limits.youngHeapMb },
        execArgv: [], env: {}, stdout: true, stderr: true,
      });
    } catch {
      return this.#finishPreparation(preparation, { kind: "failed", reason: "worker" });
    }
    worker.stdout.resume(); worker.stderr.resume();
    const job: Job = { ...preparation, worker, signal, timer: undefined,
      onAbort: () => this.stop(job, { kind: "stopped", reason: "cancelled" }) };
    this.#state = { kind: "running", job };
    worker.on("error", () => this.stop(job, { kind: "failed", reason: "worker" }));
    worker.on("exit", () => this.stop(job, { kind: "failed", reason: "worker" }));
    worker.on("message", (message: unknown) => {
      if (this.#state.kind !== "running" || this.#state.job !== job) return;
      if (message === "result-bytes") { this.stop(job, { kind: "failed", reason: "result-bytes" }); return; }
      if (request.kind === "contacts" && (message === "load" || message === "numeric" || message === "bounds")) {
        this.stop(job, { kind: "failed", reason: message }); return;
      }
      if (typeof message !== "string") { this.stop(job, { kind: "failed", reason: "worker" }); return; }
      const messageBytes = Buffer.byteLength(message, "utf8");
      if (messageBytes > this.#limits.resultBytes
        || messageBytes > this.#limits.totalResultBytes - this.#used.resultBytes) {
        this.stop(job, { kind: "failed", reason: "result-bytes" }); return;
      }
      this.#used.resultBytes += messageBytes;
      if (performance.now() >= deadline) { this.stop(job, { kind: "stopped", reason: "deadline" }); return; }
      try {
        const value: unknown = JSON.parse(message);
        const accepted: WorkResult = request.kind === "contacts"
          ? { kind: "completed", analysis: contactAccept.call(request.input, value) }
          : { kind: "completed", geometry: request.plan instanceof GeometryExecutionInput
            ? executionAccept.call(request.plan, value) : request.plan.accept(value) };
        this.stop(job, performance.now() >= deadline ? { kind: "stopped", reason: "deadline" } : accepted);
      }
      catch { this.stop(job, { kind: "failed", reason: request.kind === "contacts" ? "analysis" : "geometry" }); }
    });
    job.timer = setTimeout(() => this.stop(job, { kind: "stopped", reason: "deadline" }), Math.max(0, deadline - performance.now()));
    signal?.addEventListener("abort", job.onAbort, { once: true });
    if (job.outcome) this.stop(job, job.outcome);
    else if (signal?.aborted) job.onAbort();
    return result;
  }

  /** Supersede first; start its replacement only after this promise releases the worker. */
  async supersede(): Promise<void> {
    if (this.#state.kind === "preparing") {
      const { preparation } = this.#state;
      preparation.outcome ??= { kind: "stopped", reason: "superseded" };
      await preparation.result;
    } else if (this.#state.kind === "running" || this.#state.kind === "stopping") {
      const { job } = this.#state;
      this.stop(job, { kind: "stopped", reason: "superseded" });
      await job.result;
      if (this.#state.kind === "stopping") throw new Error("Scene worker termination failed");
    }
  }

  async close(): Promise<void> {
    if (this.#state.kind === "idle") this.#state = { kind: "closed" };
    else if (this.#state.kind === "preparing") {
      const { preparation } = this.#state;
      preparation.destination = "closed";
      preparation.outcome ??= { kind: "stopped", reason: "shutdown" };
      await preparation.result;
    }
    else if (this.#state.kind !== "closed") {
      const { job } = this.#state;
      job.destination = "closed";
      this.stop(job, { kind: "stopped", reason: "shutdown" });
      await job.result;
      if (this.#state.kind === "stopping") throw new Error("Scene worker termination failed");
    }
  }

  #finishPreparation(preparation: Preparation, fallback: WorkResult): Promise<WorkResult> {
    if (this.#state.kind !== "preparing" || this.#state.preparation !== preparation) return preparation.result;
    const outcome = preparation.outcome ?? fallback;
    this.#chargeActive(preparation.started);
    this.#state = { kind: preparation.destination };
    preparation.resolve(outcome);
    return preparation.result;
  }

  private stop(job: Job, result: WorkResult): void {
    if ((this.#state.kind !== "running" && this.#state.kind !== "stopping") || this.#state.job !== job) return;
    if (this.#state.kind === "stopping") {
      if (job.outcome?.kind === "completed" && result.kind === "stopped") job.outcome = result;
      return;
    }
    job.outcome = result;
    this.#state = { kind: "stopping", job };
    clearTimeout(job.timer);
    void job.worker.terminate().then(() => {
      job.signal?.removeEventListener("abort", job.onAbort);
      this.#chargeActive(job.started);
      this.#state = { kind: job.destination };
      job.resolve(job.outcome!);
    }, () => {
      job.signal?.removeEventListener("abort", job.onAbort);
      this.#chargeActive(job.started);
      // A termination failure cannot certify resource release or admit a new job.
      job.resolve({ kind: "failed", reason: "termination" });
    });
  }

  #chargeActive(started: number): void {
    const elapsed = Math.max(0, performance.now() - started);
    this.#used.activeMs = Math.min(this.#limits.totalActiveMs, this.#used.activeMs + elapsed);
  }
}
