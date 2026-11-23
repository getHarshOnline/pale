import validators from "./generated/validators.js";
import type { ErrorObject, ValidateFunction } from "./generated/validator-types.js";
import type { ContractByFamily } from "./generated/contract-map.js";
export type SchemaFamily = keyof typeof validators;
export interface ValidationFailure { readonly valid: false; readonly errors: readonly ErrorObject[] }
export interface ValidationSuccess<T> { readonly valid: true; readonly value: T }
/** Validate an unknown value against one exact current schema family. */
export function validate<F extends SchemaFamily>(family: F, value: unknown): ValidationSuccess<ContractByFamily[F]> | ValidationFailure {
  const validator = validators[family] as ValidateFunction;
  if (validator(value)) return { valid: true, value: value as ContractByFamily[F] };
  return { valid: false, errors: validator.errors ?? [] };
}
