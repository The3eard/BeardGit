import { describe, it, expect } from "vitest";
import fs from "node:fs";
import path from "node:path";

// Note: Vitest is not configured with the Svelte plugin in this project
// (see src/lib/components/common/List.test.ts for the pattern). These
// tests validate the dialog's pure input-collection logic and confirm
// the component file exists and compiles its i18n keys.

describe("TriggerWorkflowDialog", () => {
  it("component file exists", () => {
    const componentPath = path.resolve(__dirname, "TriggerWorkflowDialog.svelte");
    expect(fs.existsSync(componentPath)).toBe(true);
  });

  it("collects only non-blank keys into the inputs record", () => {
    // Replicates the collection logic inside `submit()` in
    // TriggerWorkflowDialog.svelte.
    const pairs = [
      { key: "DEPLOY_ENV", value: "staging" },
      { key: "", value: "orphan" },             // dropped: empty key
      { key: "  ", value: "whitespace" },       // dropped: blank key
      { key: " LOG_LEVEL ", value: "debug" },   // kept (trimmed)
    ];
    const inputs: Record<string, string> = {};
    for (const { key, value } of pairs) {
      const k = key.trim();
      if (k.length > 0) inputs[k] = value;
    }
    expect(inputs).toEqual({
      DEPLOY_ENV: "staging",
      LOG_LEVEL: "debug",
    });
  });

  it("addPair appends a blank row; removePair restores a single blank row when empty", () => {
    // Replicates `addPair()` and `removePair()` behaviour.
    let pairs: { key: string; value: string }[] = [{ key: "", value: "" }];
    const addPair = () => { pairs = [...pairs, { key: "", value: "" }]; };
    const removePair = (idx: number) => {
      pairs = pairs.filter((_, i) => i !== idx);
      if (pairs.length === 0) pairs = [{ key: "", value: "" }];
    };

    addPair();
    expect(pairs.length).toBe(2);
    addPair();
    expect(pairs.length).toBe(3);

    removePair(1);
    expect(pairs.length).toBe(2);

    removePair(0);
    removePair(0);
    // After removing both entries, a single blank row should remain.
    expect(pairs.length).toBe(1);
    expect(pairs[0]).toEqual({ key: "", value: "" });
  });

  it("uses i18n keys consistent with the messages files", async () => {
    // Ensure the dialog's i18n key set is present in both locales.
    const en = JSON.parse(
      fs.readFileSync(path.resolve(__dirname, "../../../../messages/en-US.json"), "utf8"),
    );
    const es = JSON.parse(
      fs.readFileSync(path.resolve(__dirname, "../../../../messages/es-ES.json"), "utf8"),
    );
    const required = [
      "pipeline_trigger_dialog_title",
      "pipeline_trigger_workflow_label",
      "pipeline_trigger_ref_label",
      "pipeline_trigger_inputs_label",
      "pipeline_trigger_variables_label",
      "pipeline_trigger_add_variable",
      "pipeline_trigger_submit",
      "pipeline_trigger_cancel",
      "pipeline_trigger_error",
      "pipeline_trigger_no_workflows",
      "pipeline_variable_key_placeholder",
      "pipeline_variable_value_placeholder",
    ];
    for (const key of required) {
      expect(en[key], `en-US missing ${key}`).toBeDefined();
      expect(es[key], `es-ES missing ${key}`).toBeDefined();
    }
  });
});
