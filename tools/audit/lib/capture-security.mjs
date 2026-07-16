import crypto from 'node:crypto';
import fs from 'node:fs';

const SENSITIVE_CARRIER = /(?:proxy-)?authorization\s*:\s*(?:basic|bearer)\s+\S+|\b(?:basic|bearer)\s+\S+|(?:^|[;\s])cookie\s*:\s*\S+|\b(?:api[-_]?key|password|secret|token)\s*[=:]\s*\S+/i;

export function containsSensitiveCarrier(value) {
  return SENSITIVE_CARRIER.test(String(value));
}

export function executableDigest(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

export function assertStableExecutableDigest(filePath, expectedDigest) {
  if (executableDigest(filePath) !== expectedDigest) {
    throw new Error('capture executable changed during execution; digest is not stable');
  }
}
