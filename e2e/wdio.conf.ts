import type { Options } from "@wdio/types";
import path from "path";

export const config: Options.Testrunner = {
  runner: "local",
  specs: ["./e2e/specs/**/*.spec.ts"],
  maxInstances: 1,
  capabilities: [{
    "tauri:options": {
      application: path.resolve("./src-tauri/target/debug/beardgit"),
    },
  }],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
};
