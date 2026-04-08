/**
 * Git utility functions shared across components.
 */

/**
 * Return the first 7 characters of a full OID for display purposes.
 */
export function shortOid(oid: string): string {
  return oid.substring(0, 7);
}
