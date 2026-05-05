/**
 * Unit tests for the `lintBuffer` JSON linter.
 *
 * Drives the pure helper directly (no `EditorView`) so we can assert
 * on the diagnostics array produced for malformed input and a couple
 * of schema rules. We don't exercise full schema validation — just
 * "is the chassis wired up correctly?".
 */

import { describe, expect, it } from "vitest";
import { lintBuffer } from "../json-lint";

describe("lintBuffer", () => {
  it("emits no diagnostics for an empty buffer", () => {
    expect(lintBuffer("", "package.json")).toEqual([]);
    expect(lintBuffer("   \n", "package.json")).toEqual([]);
  });

  it("emits no diagnostics for valid JSON without schema rules", () => {
    expect(lintBuffer('{"hello": "world"}', "any.json")).toEqual([]);
  });

  it("emits a parse-error diagnostic for malformed JSON", () => {
    const diagnostics = lintBuffer("{ not: valid }", "any.json");
    expect(diagnostics.length).toBeGreaterThanOrEqual(1);
    const [first] = diagnostics;
    expect(first.severity).toBe("error");
    expect(first.source).toBe("json");
    expect(first.from).toBeGreaterThanOrEqual(0);
    expect(first.to).toBeGreaterThanOrEqual(first.from);
  });

  it("warns when package.json is missing `name` / `version`", () => {
    const diagnostics = lintBuffer("{}", "package.json");
    const messages = diagnostics.map((d) => d.message).join("\n");
    expect(messages).toMatch(/name/);
    expect(messages).toMatch(/version/);
    expect(diagnostics.every((d) => d.severity === "warning")).toBe(true);
  });

  it("does not warn when package.json declares both name and version", () => {
    const diagnostics = lintBuffer(
      '{"name":"x","version":"0.0.1"}',
      "/repo/package.json",
    );
    expect(diagnostics).toEqual([]);
  });

  it("flags tsconfig.json with a non-object compilerOptions", () => {
    const diagnostics = lintBuffer(
      '{"compilerOptions": "oops"}',
      "tsconfig.json",
    );
    expect(diagnostics.length).toBe(1);
    expect(diagnostics[0].severity).toBe("error");
    expect(diagnostics[0].message).toMatch(/compilerOptions/);
  });

  it("flags non-scalar values in a Requests env file", () => {
    const filename = "/repo/.beardgit/requests/_env/dev.json";
    const diagnostics = lintBuffer(
      '{"BASE":"http://x","DEEP":{"a":1}}',
      filename,
    );
    expect(diagnostics.length).toBe(1);
    expect(diagnostics[0].message).toMatch(/DEEP/);
  });
});
