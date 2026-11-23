const unsignedInteger = (maximum) => ({
  type: "number",
  validate: (value) => Number.isSafeInteger(value) && value >= 0 && value <= maximum,
});

export const UINT_FORMATS = Object.freeze({
  uint16: unsignedInteger(0xffff),
  uint32: unsignedInteger(0xffff_ffff),
  uint64: unsignedInteger(Number.MAX_SAFE_INTEGER),
});
