import { describe, it, expect } from "vitest";
import { formatSigningBackend, signatureChipState } from "./signing";
import type { CommitSignature, SignatureVerification } from "$lib/types";

describe("formatSigningBackend", () => {
  it("maps ssh and x509 to display labels", () => {
    expect(formatSigningBackend("ssh")).toBe("SSH");
    expect(formatSigningBackend("x509")).toBe("X.509");
  });

  it("falls back to GPG for openpgp, unknown, and null", () => {
    expect(formatSigningBackend("openpgp")).toBe("GPG");
    expect(formatSigningBackend("gpg")).toBe("GPG");
    expect(formatSigningBackend(null)).toBe("GPG");
    expect(formatSigningBackend(undefined)).toBe("GPG");
  });
});

describe("signatureChipState", () => {
  const signed: CommitSignature = { present: true, format: "ssh" };
  const unsigned: CommitSignature = { present: false, format: null };
  const verified: SignatureVerification = { status: "verified", detail: "good" };
  const unverified: SignatureVerification = { status: "unverified", detail: "no signer" };

  it("returns none when unsigned or not yet loaded", () => {
    expect(signatureChipState(null, null, false)).toBe("none");
    expect(signatureChipState(unsigned, null, false)).toBe("none");
  });

  it("returns signed when present but not yet verified", () => {
    expect(signatureChipState(signed, null, false)).toBe("signed");
  });

  it("returns verifying while a verify is in flight (takes precedence)", () => {
    expect(signatureChipState(signed, null, true)).toBe("verifying");
    // Even with a cached verdict, an in-flight re-verify shows verifying.
    expect(signatureChipState(signed, verified, true)).toBe("verifying");
  });

  it("reflects the verification verdict when settled", () => {
    expect(signatureChipState(signed, verified, false)).toBe("verified");
    expect(signatureChipState(signed, unverified, false)).toBe("unverified");
  });

  it("never shows a chip state for an unsigned commit even with a verdict", () => {
    expect(signatureChipState(unsigned, verified, false)).toBe("none");
  });
});
