// Schemars emits Rust integer widths as formats. JSON numbers must remain exact
// in JavaScript; editor u64 coordinates instead travel as decimal strings.
const unsigned = (maximum) => ({
  type: "number",
  validate: (value) => Number.isSafeInteger(value) && value >= 0 && value <= maximum,
});

export const numberFormats = Object.freeze({
  uint16: unsigned(0xffff),
  uint32: unsigned(0xffff_ffff),
  // Bound the text before BigInt; never round an editor coordinate through Number.
  "uint64-decimal": {
    type: "string",
    validate: (value) => typeof value === "string"
      && value.length >= 1 && value.length <= 20
      && /^(0|[1-9][0-9]*)$/u.test(value)
      && BigInt(value) <= 0xffff_ffff_ffff_ffffn,
  },
});
