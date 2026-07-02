/**
 * Pure helpers for the commit-signing UI (commit-box chip, settings, and the
 * commit-detail signature chip). Kept framework-free so they can be unit
 * tested and shared across the three call sites.
 */
import type { CommitSignature, SignatureVerification } from "$lib/types";

/**
 * Human label for a signing backend. Accepts a raw `gpg.format` value
 * (`"ssh"`, `"x509"`, `"openpgp"`, `null`) or the `format` hint sniffed from a
 * signature; anything not explicitly ssh/x509 falls back to GPG (git's default).
 */
export function formatSigningBackend(format: string | null | undefined): string {
  if (format === "ssh") return "SSH";
  if (format === "x509") return "X.509";
  return "GPG";
}

/** Discrete state of the commit-detail signature chip. */
export type SignatureChipState =
  | "none"
  | "verifying"
  | "verified"
  | "unverified"
  | "signed";

/**
 * Resolve which chip state to render for the open commit. `none` means no chip
 * (unsigned / not yet loaded); the others map 1:1 to an i18n label in the
 * component. Verification is optional (lazy) — a present-but-unverified
 * signature shows `signed` until the verdict lands.
 */
export function signatureChipState(
  signature: CommitSignature | null,
  verification: SignatureVerification | null,
  verifying: boolean,
): SignatureChipState {
  if (!signature?.present) return "none";
  if (verifying) return "verifying";
  if (verification?.status === "verified") return "verified";
  if (verification?.status === "unverified") return "unverified";
  return "signed";
}
