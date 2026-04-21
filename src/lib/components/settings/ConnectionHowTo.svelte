<!--
  ConnectionHowTo — compact top-of-page dropdown for the Integrations
  category (Spec 4 Phase 7).

  Was: a bordered, standalone collapsible card whose body dumped the
  entire OAuth / PAT / CLI walkthrough at once.

  Now: a flush inline control — a "How to connect:" label plus a <select>
  with three methods (OAuth / PAT / CLI). Body stays collapsed until the
  user toggles the chevron; once open, it shows only the section for the
  chosen method. A trailing troubleshooting block is rendered regardless
  of mode, because each troubleshooting item applies to every path.

  Lives as the Integrations page's very first child — no enclosing Card
  — so the page hierarchy is "howto → Connections Card". See
  `IntegrationsSettings.svelte`.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";

  type Mode = "oauth" | "pat" | "cli";

  let open = $state(false);
  let mode = $state<Mode>("oauth");

  function toggle() {
    open = !open;
  }

  function onModeChange(e: Event) {
    const value = (e.currentTarget as HTMLSelectElement).value as Mode;
    mode = value;
    // Opening the body on mode change feels natural — the user just
    // expressed intent to read a specific walkthrough.
    open = true;
  }
</script>

<div class="howto-inline" data-testid="integrations-howto">
  <div class="howto-control">
    <button
      class="toggle"
      type="button"
      aria-expanded={open}
      onclick={toggle}
      title={m.connection_howto_title()}
    >
      <span class="chevron nf" class:open>&#xF105;</span>
      <span class="toggle-label">{m.connection_howto_title()}</span>
    </button>

    <label class="mode-label" for="howto-mode">
      {m.settings_integrations_howto_label()}
    </label>
    <select
      id="howto-mode"
      class="bg-select mode-select"
      value={mode}
      onchange={onModeChange}
    >
      <option value="oauth"
        >{m.settings_integrations_howto_option_oauth()}</option
      >
      <option value="pat"
        >{m.settings_integrations_howto_option_pat()}</option
      >
      <option value="cli"
        >{m.settings_integrations_howto_option_cli()}</option
      >
    </select>
  </div>

  {#if open}
    <div class="howto-body">
      <p class="lead">{m.connection_howto_lead()}</p>

      {#if mode === "oauth"}
        <h4 class="path-title">
          {m.connection_howto_selfhosted_oauth_title()}
        </h4>
        <p>{m.connection_howto_selfhosted_description()}</p>
        <p>{m.connection_howto_selfhosted_oauth_body()}</p>
        <pre><code>glab config set client_id &lt;client_id&gt; -g --host &lt;your-gitlab-host&gt;
glab auth login --hostname &lt;your-gitlab-host&gt;</code></pre>
      {:else if mode === "pat"}
        <h4 class="path-title">
          {m.connection_howto_standard_title()}
        </h4>
        <p>{m.connection_howto_standard_description()}</p>
        <ul>
          <li>
            <strong>{m.connection_howto_standard_token_label()}</strong>
            {m.connection_howto_standard_token_body()}
            <ul>
              <li>
                GitHub: <a
                  href="https://github.com/settings/tokens"
                  target="_blank"
                  rel="noopener noreferrer"
                  >github.com/settings/tokens</a
                >
                — {m.connection_howto_scopes_github()}
              </li>
              <li>
                GitLab: <a
                  href="https://gitlab.com/-/user_settings/personal_access_tokens"
                  target="_blank"
                  rel="noopener noreferrer"
                  >gitlab.com/-/user_settings/personal_access_tokens</a
                >
                — {m.connection_howto_scopes_gitlab()}
              </li>
            </ul>
          </li>
        </ul>

        <h5 class="sub-title">
          {m.connection_howto_selfhosted_token_title()}
        </h5>
        <p>{m.connection_howto_selfhosted_token_body()}</p>
        <pre><code>glab auth login --hostname &lt;your-gitlab-host&gt; --token &lt;your-PAT&gt; --api-protocol https</code></pre>
        <p class="small">{m.connection_howto_selfhosted_token_scope_hint()}</p>
      {:else}
        <h4 class="path-title">
          {m.connection_howto_standard_title()}
        </h4>
        <p>{m.connection_howto_standard_cli_body()}</p>
        <pre><code>gh auth login
glab auth login</code></pre>
      {/if}

      <h4 class="path-title">{m.connection_howto_troubleshoot_title()}</h4>
      <ul>
        <li>
          <strong
            >{m.connection_howto_troubleshoot_multi_config_label()}</strong
          >
          {m.connection_howto_troubleshoot_multi_config_body()}
          <pre><code>cat ~/.config/glab-cli/config.yml
cat ~/Library/Application\ Support/glab-cli/config.yml</code></pre>
          {m.connection_howto_troubleshoot_multi_config_fix()}
        </li>
        <li>
          <strong>{m.connection_howto_troubleshoot_verify_label()}</strong>
          <pre><code>gh auth status
glab auth status</code></pre>
        </li>
        <li>
          <strong>{m.connection_howto_troubleshoot_404_label()}</strong>
          {m.connection_howto_troubleshoot_404_body()}
        </li>
      </ul>
    </div>
  {/if}
</div>

<style>
  .howto-inline {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
    /* flush in the page — no border/background. */
  }

  .howto-control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    font: inherit;
    padding: 4px 6px 4px 0;
  }

  .toggle:hover {
    color: var(--accent-blue);
  }

  .chevron {
    display: inline-block;
    transition: transform 0.15s ease;
    font-size: 11px;
    color: var(--text-secondary);
    width: 12px;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .toggle-label {
    font-weight: 600;
    font-size: 13px;
  }

  .mode-label {
    color: var(--text-secondary);
    font-size: 12px;
    margin-left: 8px;
  }

  .mode-select {
    min-width: 220px;
    padding: 4px 8px;
    font-size: 12px;
    border-radius: 4px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  .mode-select:focus {
    border-color: var(--accent-blue);
    outline: none;
  }

  .howto-body {
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-secondary);
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-primary);
  }

  .lead {
    color: var(--text-secondary);
    margin: 4px 0 14px 0;
  }

  .path-title {
    margin: 18px 0 6px 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent-blue);
  }

  .path-title:first-of-type {
    margin-top: 4px;
  }

  .sub-title {
    margin: 14px 0 4px 0;
    font-size: 12.5px;
    color: var(--text-primary);
  }

  ul {
    margin: 4px 0;
    padding-left: 18px;
  }

  ul ul {
    margin: 4px 0 6px 0;
  }

  li {
    margin: 4px 0;
  }

  pre {
    margin: 6px 0;
    padding: 8px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    overflow-x: auto;
    color: var(--text-primary);
  }

  code {
    font-family: var(--font-mono, monospace);
  }

  a {
    color: var(--accent-blue);
    text-decoration: none;
  }

  a:hover {
    text-decoration: underline;
  }

  .small {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 4px 0 0 0;
  }
</style>
