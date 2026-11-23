/** JSON values accepted by the RFC 8785 canonicalizer. */
export type JsonValue = null | boolean | number | string | JsonValue[] | { readonly [key: string]: JsonValue };

function requireUnicodeScalarText(value: string): void {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const following = value.charCodeAt(index + 1);
      if (!(following >= 0xdc00 && following <= 0xdfff)) throw new TypeError("JCS does not admit lone Unicode surrogates");
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw new TypeError("JCS does not admit lone Unicode surrogates");
    }
  }
}

/** Canonicalize one JSON value according to RFC 8785 (JCS). */
export function canonicalize(value: JsonValue): string {
  if (value === null || typeof value === "boolean") return JSON.stringify(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new TypeError("JCS does not admit non-finite numbers");
    return JSON.stringify(value);
  }
  if (typeof value === "string") { requireUnicodeScalarText(value); return JSON.stringify(value); }
  if (Array.isArray(value)) {
    if (Object.keys(value).length !== value.length) throw new TypeError("JCS does not admit sparse arrays");
    return `[${value.map(canonicalize).join(",")}]`;
  }
  if (Object.getPrototypeOf(value) !== Object.prototype) throw new TypeError("JCS objects must be ordinary JSON objects");
  return `{${Object.keys(value).sort().map((key) => { requireUnicodeScalarText(key); return `${JSON.stringify(key)}:${canonicalize(value[key]!)}`; }).join(",")}}`;
}
