/** @aiPurpose Generate and validate cloud references shared by Node and browser consumers. */
const alphabet = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
const width = 12;
const compactPattern = /^[A-Za-z0-9]{12}$/;
export const cloudTokenPattern = compactPattern;
export const cloudIncarnationPattern = compactPattern;
export const cloudWorkspaceIdPattern = /^WS[A-Za-z0-9]{10}$/;

export function assertCloudIncarnation(value: unknown): void {
  if (typeof value !== 'string' || !cloudIncarnationPattern.test(value)) {
    throw new TypeError('Cloud Agent incarnation must contain 12 ASCII letters or digits');
  }
}

export function assertCloudWorkspaceId(value: unknown): void {
  if (typeof value !== 'string' || !cloudWorkspaceIdPattern.test(value)) {
    throw new TypeError('Cloud workspace ID must be WS followed by 10 ASCII letters or digits');
  }
}

/** Check compact cloud fields in decoded responses, outgoing bodies and query arguments. */
export function assertCloudIdentifiers(value: unknown, response = false): void {
  function check(value: unknown, messageEndpoint = false): void {
    if (!value || typeof value !== 'object') return;
    if (Array.isArray(value)) {
      for (const item of value) check(item);
      return;
    }
    for (const [key, field] of Object.entries(value)) {
      if (field === undefined) continue;
      if (key === 'workspace_id') assertCloudWorkspaceId(field);
      else if (key === 'incarnation') {
        // The synthetic human endpoint is a message participant, never an Agent registration.
        if (messageEndpoint && field === '~human' && 'host' in value && value.host === '~space'
          && 'name' in value && value.name === 'operator') continue;
        assertCloudIncarnation(field);
      } else check(field, response && (key === 'from' || key === 'to'));
    }
  }
  check(value);
}

/** Generate a case-sensitive cloud request or bearer using unbiased Web Crypto bytes. */
export function newCloudId(): string {
  let value = '';
  const bytes = new Uint8Array(16);
  while (value.length < width) {
    crypto.getRandomValues(bytes);
    for (const byte of bytes) {
      if (byte >= 248) continue;
      value += alphabet[byte % alphabet.length];
      if (value.length === width) break;
    }
  }
  return value;
}
