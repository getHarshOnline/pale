import { canonicalize, type JsonValue } from "./jcs.js";
import { AUTHORED_INPUT_FAMILIES, type AuthoredInputFamily } from "./generated/registry.js";
import type { ContractByFamily } from "./generated/contract-map.js";
import { validate, type ValidationFailure } from "./validation.js";

/** Recursively immutable view of one admitted JSON-shaped contract. */
export type DeepReadonly<T> = T extends JsonValue[]
  ? readonly DeepReadonly<T[number]>[]
  : T extends object
    ? { readonly [K in keyof T]: DeepReadonly<T[K]> }
    : T;

/** Structurally admitted candidate bound to its exact canonical bytes. */
export interface BuiltCandidate<F extends AuthoredInputFamily> {
  readonly family: F;
  readonly value: DeepReadonly<ContractByFamily[F]>;
  readonly canonicalJson: string;
}

export class CandidateValidationError extends Error {
  constructor(readonly family: AuthoredInputFamily, readonly failures: ValidationFailure["errors"]) { super(`candidate does not satisfy ${family}`); this.name = "CandidateValidationError"; }
}

/** Copy one JSON tree so caller-owned aliases cannot mutate admitted state. */
function ownJson(value: JsonValue): JsonValue {
  if (value === null || typeof value !== "object") return value;
  if (Array.isArray(value)) {
    const keys = Object.keys(value);
    if (keys.length !== value.length || keys.some((key, index) => key !== String(index))) {
      throw new TypeError("Pale candidates do not admit sparse arrays or named array properties");
    }
    return value.map(ownJson);
  }
  if (Object.getPrototypeOf(value) !== Object.prototype) {
    throw new TypeError("Pale candidates must contain only ordinary JSON objects");
  }
  const owned: { [key: string]: JsonValue } = {};
  const descriptors = Object.getOwnPropertyDescriptors(value);
  for (const key of Object.keys(value)) {
    const descriptor = descriptors[key]!;
    if (!("value" in descriptor)) {
      throw new TypeError("Pale candidates do not admit accessor properties");
    }
    owned[key] = ownJson(descriptor.value as JsonValue);
  }
  return owned;
}

/** Freeze every container in one already-owned JSON tree. */
function freezeJson<T extends JsonValue>(value: T): DeepReadonly<T> {
  if (value !== null && typeof value === "object") {
    for (const child of Array.isArray(value) ? value : Object.values(value)) freezeJson(child);
    Object.freeze(value);
  }
  return value as DeepReadonly<T>;
}

/** Build a structurally valid, effect-free candidate for an authored input. */
export function buildAuthoredInput<F extends AuthoredInputFamily>(family: F, value: ContractByFamily[F]): BuiltCandidate<F> {
  if (!(AUTHORED_INPUT_FAMILIES as readonly string[]).includes(family)) throw new TypeError(`${family as string} is not an authored-input family`);
  const owned = ownJson(value as JsonValue) as unknown as ContractByFamily[F];
  const result = validate(family, owned);
  if (!result.valid) throw new CandidateValidationError(family, result.errors);
  const canonicalJson = canonicalize(owned as JsonValue);
  const frozen = freezeJson(owned as JsonValue) as unknown as DeepReadonly<ContractByFamily[F]>;
  return Object.freeze({ family, value: frozen, canonicalJson });
}
