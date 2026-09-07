import { describe, it, expect } from "vitest";
import { encodeTerminalInput } from "./input-encoding";

function decode(b64: string): number[] {
  return Array.from(atob(b64), (c) => c.charCodeAt(0));
}

describe("encodeTerminalInput", () => {
  it("passes ASCII control bytes through (Ctrl+R is 0x12)", () => {
    expect(decode(encodeTerminalInput("\x12"))).toEqual([0x12]);
    expect(decode(encodeTerminalInput("ls\r"))).toEqual([0x6c, 0x73, 0x0d]);
  });

  it("encodes Latin-1 range characters as UTF-8, not as a single byte", () => {
    expect(decode(encodeTerminalInput("ñ"))).toEqual([0xc3, 0xb1]);
  });

  it("does not throw on characters above U+00FF", () => {
    expect(decode(encodeTerminalInput("€"))).toEqual([0xe2, 0x82, 0xac]);
    expect(decode(encodeTerminalInput("😀"))).toEqual([0xf0, 0x9f, 0x98, 0x80]);
  });
});
