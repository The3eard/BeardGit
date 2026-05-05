<!--
  AuthTab.svelte — High-level authentication picker.

  Re-writes the request's headers in place when the user changes auth
  type or its inputs. `Authorization` and (for API keys) the chosen
  header name are stripped first so switching strategies doesn't leave
  stale headers behind. "Custom" is an explicit escape hatch that
  preserves whatever the user has set on the Headers tab.

  Uses the shared `Field` primitive plus the canonical `bg-select` /
  `bg-input` recipes so the auth controls render identically to other
  forms in the app (settings, repo-config, etc.).
-->
<script lang="ts">
  import { Field } from "$lib/components/ui";
  import { currentRequest } from "./stores";

  /** Supported auth strategies. */
  type AuthKind = "none" | "basic" | "bearer" | "apikey" | "custom";
  /** Active strategy in the dropdown. */
  let kind: AuthKind = "none";
  /** Bearer token value, used when `kind === "bearer"`. */
  let bearer = "";
  /** Header name for API key auth (e.g. `X-API-Key`). */
  let apiKeyName = "X-API-Key";
  /** Header value for API key auth. */
  let apiKeyValue = "";
  /** Username for HTTP basic auth. */
  let user = "";
  /** Password for HTTP basic auth. */
  let pass = "";

  /**
   * Strip any existing Authorization / API-key header and add the one
   * implied by the current `kind`. No-ops when no request is loaded.
   */
  function apply() {
    if (!$currentRequest) return;
    const headers = $currentRequest.headers.filter(
      ([k]) =>
        k.toLowerCase() !== "authorization" &&
        k.toLowerCase() !== apiKeyName.toLowerCase(),
    );
    switch (kind) {
      case "bearer":
        headers.push(["Authorization", `Bearer ${bearer}`]);
        break;
      case "basic":
        headers.push(["Authorization", `Basic ${btoa(`${user}:${pass}`)}`]);
        break;
      case "apikey":
        headers.push([apiKeyName, apiKeyValue]);
        break;
      default:
        break;
    }
    currentRequest.set({ ...$currentRequest, headers });
  }
</script>

<div class="auth">
  <Field label="Type" for="auth-kind">
    <select
      id="auth-kind"
      class="bg-select"
      bind:value={kind}
      on:change={apply}
    >
      <option value="none">None</option>
      <option value="basic">Basic</option>
      <option value="bearer">Bearer</option>
      <option value="apikey">API Key</option>
      <option value="custom">Custom (use Headers tab)</option>
    </select>
  </Field>

  {#if kind === "bearer"}
    <Field label="Token" for="auth-bearer">
      <input
        id="auth-bearer"
        class="bg-input"
        placeholder="token (or {'{{var}}'})"
        bind:value={bearer}
        on:input={apply}
      />
    </Field>
  {/if}

  {#if kind === "basic"}
    <Field label="User" for="auth-user">
      <input
        id="auth-user"
        class="bg-input"
        placeholder="user"
        bind:value={user}
        on:input={apply}
      />
    </Field>
    <Field label="Password" for="auth-pass">
      <input
        id="auth-pass"
        class="bg-input"
        type="password"
        placeholder="pass"
        bind:value={pass}
        on:input={apply}
      />
    </Field>
  {/if}

  {#if kind === "apikey"}
    <Field label="Header name" for="auth-apikey-name">
      <input
        id="auth-apikey-name"
        class="bg-input"
        placeholder="header name"
        bind:value={apiKeyName}
        on:input={apply}
      />
    </Field>
    <Field label="Value" for="auth-apikey-value">
      <input
        id="auth-apikey-value"
        class="bg-input"
        placeholder="value"
        bind:value={apiKeyValue}
        on:input={apply}
      />
    </Field>
  {/if}
</div>

<style>
  .auth {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 360px;
  }

  .bg-select,
  .bg-input {
    width: 100%;
    height: 30px;
    line-height: 28px;
    padding: 0 10px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
  }

  .bg-select {
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    cursor: pointer;
    padding-right: 26px;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'><path fill='none' stroke='%23888' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round' d='M1 1l4 4 4-4'/></svg>");
    background-repeat: no-repeat;
    background-position: right 10px center;
  }

  .bg-select:focus,
  .bg-input:focus {
    border-color: var(--accent-blue);
  }
</style>
