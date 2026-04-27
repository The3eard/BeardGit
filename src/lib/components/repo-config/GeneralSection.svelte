<!--
  GeneralSection.svelte — description / homepage / topics editor.

  Writes directly into `repoConfigStore.current` via `updateCurrent()`
  so the diff-driven Save button in the dialog shell reflects the edits
  in real time. The topic chip input follows the convention used
  elsewhere in the app (Issues labels, MR reviewers): Enter commits the
  chip, Backspace on an empty input removes the last chip.

  Every CLI argument (topic name, description, homepage URL) ends up
  in `apply_remote_repo_config` as an individual `arg()` call on the
  Rust side — the frontend does no escaping. See the spec's shell-safety
  note: the chip input may contain any characters including `; & | $`.
-->
<script lang="ts">
  import { Field, FormRow } from "$lib/components/ui";
  import { repoConfigStore, updateCurrent } from "$lib/stores/repoConfig";

  let topicInput = $state("");

  $effect(() => {
    // When the loaded config changes (e.g. the dialog opens for a
    // different repo) clear any half-typed topic so it doesn't leak
    // across sessions.
    void $repoConfigStore.repoPath;
    topicInput = "";
  });

  let current = $derived($repoConfigStore.current);

  function setDescription(value: string) {
    updateCurrent((draft) => {
      draft.description = value;
    });
  }

  function setHomepage(value: string) {
    updateCurrent((draft) => {
      // Empty string maps to `null` on the backend — the store's diff
      // logic then emits `PatchValue::Clear` so the forge clears it.
      draft.homepage = value.length === 0 ? null : value;
    });
  }

  function addTopic(raw: string) {
    const topic = raw.trim();
    if (!topic) return;
    updateCurrent((draft) => {
      if (!draft.topics.includes(topic)) {
        draft.topics = [...draft.topics, topic];
      }
    });
  }

  function removeTopic(topic: string) {
    updateCurrent((draft) => {
      draft.topics = draft.topics.filter((t) => t !== topic);
    });
  }

  function handleTopicKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      addTopic(topicInput);
      topicInput = "";
    } else if (e.key === "Backspace" && topicInput.length === 0) {
      const topics = current?.topics ?? [];
      if (topics.length > 0) {
        removeTopic(topics[topics.length - 1]);
      }
    }
  }
</script>

<div class="repo-config-general" data-testid="repo-config-general">
  {#if current}
    <Field
      label="Description"
      description="Shown at the top of the repo page on the forge."
      for="repo-config-description"
    >
      <textarea
        id="repo-config-description"
        class="bg-textarea"
        rows="3"
        value={current.description}
        oninput={(e) => setDescription((e.target as HTMLTextAreaElement).value)}
        data-testid="repo-config-description"
      ></textarea>
    </Field>

    <Field
      label="Homepage"
      description="Public URL linked from the repo header. Leave blank to clear."
      for="repo-config-homepage"
    >
      <input
        id="repo-config-homepage"
        type="url"
        class="bg-input"
        placeholder="https://example.com"
        value={current.homepage ?? ""}
        oninput={(e) => setHomepage((e.target as HTMLInputElement).value)}
        data-testid="repo-config-homepage"
      />
    </Field>

    <FormRow label="Topics" for="repo-config-topic-input">
      <div class="topics" data-testid="repo-config-topics">
        {#each current.topics as topic (topic)}
          <span class="chip" data-testid="repo-config-topic-chip">
            <span class="chip-label">{topic}</span>
            <button
              type="button"
              class="chip-remove nf"
              aria-label={`Remove topic ${topic}`}
              onclick={() => removeTopic(topic)}
            >{"\uF00D"}</button>
          </span>
        {/each}
        <input
          id="repo-config-topic-input"
          type="text"
          class="chip-input"
          placeholder="Type and press Enter"
          bind:value={topicInput}
          onkeydown={handleTopicKeydown}
          data-testid="repo-config-topic-input"
        />
      </div>
    </FormRow>
  {/if}
</div>

<style>
  .repo-config-general {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .bg-textarea,
  .bg-input {
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: inherit;
    font-size: 12px;
    line-height: 1.5;
    resize: vertical;
  }

  .bg-textarea:focus,
  .bg-input:focus {
    outline: none;
    border-color: var(--accent-blue);
  }

  .topics {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    width: 100%;
    min-height: 28px;
    padding: 4px 6px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 4px 1px 8px;
    background: var(--overlay-accent-blue);
    color: var(--text-primary);
    border-radius: 10px;
    font-size: 11px;
    line-height: 18px;
  }

  .chip-label {
    white-space: nowrap;
  }

  .chip-remove {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-icons);
    font-size: 8px;
    line-height: 1;
    padding: 2px;
  }

  .chip-remove:hover {
    color: var(--accent-red);
  }

  .chip-input {
    flex: 1;
    min-width: 100px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    padding: 2px 4px;
  }

  .chip-input:focus {
    outline: none;
  }
</style>
