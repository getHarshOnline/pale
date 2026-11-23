import { isDeepStrictEqual as equal } from 'node:util';

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

// The native adapter qualifies its stages and supplies the observed receipt.
export function verifyLocalRenderReceipt(receipt, { status, selection }) {
  if (!isRecord(receipt) || typeof status !== 'string' || !status.trim() || receipt.status !== status) {
    throw new TypeError('Local render status differs from the admitted success status');
  }
  if (!isRecord(selection) || !Object.keys(selection).length ||
      !equal(receipt.build_selection, selection)) {
    throw new TypeError('Local render selection differs from the admitted selection');
  }
  const sources = receipt.inputs?.sources;
  if (!isRecord(sources) || !Object.keys(sources).length ||
      Object.entries(sources).some(([name, source]) => !name.trim() || !isRecord(source) || !Object.keys(source).length)) {
    throw new TypeError('Local render receipt lacks a source inventory');
  }
  const identity = receipt.inventory?.identity;
  if (!isRecord(identity) || typeof identity.format !== 'string' || !identity.format.trim() ||
      typeof identity.sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(identity.sha256)) {
    throw new TypeError('Local render receipt lacks a valid output inventory identity');
  }
  return receipt;
}
