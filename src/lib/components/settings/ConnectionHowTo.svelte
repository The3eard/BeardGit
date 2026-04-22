<!--
  ConnectionHowTo — compact top-of-page dropdown for the Integrations
  category.

  Was: a bordered, standalone collapsible card whose body dumped the
  entire OAuth / PAT / CLI walkthrough at once.

  Now: a flush inline control — a "How to connect:" label plus a <select>
  with three methods (PAT / OAuth / CLI). Body stays collapsed until the
  user toggles the chevron; once open, it shows only the section for the
  chosen method. A trailing troubleshooting block is rendered regardless
  of mode, because each troubleshooting item applies to every path.

  PAT is the primary-recommended path and is selected by default: a
  successful PAT connect pipes the token into the matching CLI in the
  background, so users don't have to run a terminal step. The PAT body
  surfaces the Classic/Legacy-vs-fine-grained caveat (cli/cli#6680), SSO
  guidance, and a collapsed manual-login command reference for
  troubleshooting. Template copy puts `<YOUR_PAT>` placeholders on the
  clipboard — no real token ever touches this component.

  Lives as the Integrations page's very first child — no enclosing Card
  — so the page hierarchy is "howto → Connections Card". See
  `IntegrationsSettings.svelte`.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { Button } from "$lib/components/ui";

  type Mode = "pat" | "oauth" | "cli";

  let open = $state(false);
  let mode = $state<Mode>("pat");
  /** Which template block last received a copy, used for transient feedback. */
  let copiedKey = $state<string | null>(null);

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

  /** Copy the literal template to the clipboard with `<YOUR_PAT>` intact. */
  function copyTemplate(text: string, key: string) {
    void navigator.clipboard?.writeText(text);
    copiedKey = key;
    window.setTimeout(() => {
      if (copiedKey === key) copiedKey = null;
    }, 1500);
  }

  /**
   * Render a localized string with `**bold**` spans as `<strong>` elements.
   * Inputs come from our own i18n files (trusted), but we still HTML-escape
   * non-bold text defensively so `<`/`>` in future translations can't break
   * the DOM.
   */
  function renderBold(text: string): string {
    const escape = (s: string) =>
      s
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
    return text.replace(/\*\*([^*]+)\*\*/g, (_m, inner) => {
      return `<strong>${escape(inner)}</strong>`;
    });
  }

  // Literal manual-login templates. `<YOUR_PAT>` is a placeholder — we
  // never substitute a real token here; users paste their own in-terminal.
  const ghInsecureTemplate =
    'echo "<YOUR_PAT>" | gh auth login --with-token --hostname github.com';
  const glabInsecureTemplate =
    'echo "<YOUR_PAT>" | glab auth login --stdin --hostname gitlab.com';
  const ghSecureTemplate =
    'read -rs -p "Paste PAT: " T && echo "$T" | gh auth login --with-token --hostname github.com';
  const glabSecureTemplate =
    'read -rs -p "Paste PAT: " T && echo "$T" | glab auth login --stdin --hostname gitlab.com';
  const insecureTemplate = `${ghInsecureTemplate}\n${glabInsecureTemplate}`;
  const secureTemplate = `${ghSecureTemplate}\n${glabSecureTemplate}`;
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
      <option value="pat"
        >{m.settings_integrations_howto_option_pat()}</option
      >
      <option value="oauth"
        >{m.settings_integrations_howto_option_oauth()}</option
      >
      <option value="cli"
        >{m.settings_integrations_howto_option_cli()}</option
      >
    </select>
  </div>

  {#if open}
    <div class="howto-body">
      <p class="lead">{m.connection_howto_lead()}</p>

      {#if mode === "pat"}
        <h4 class="path-title">
          {m.connection_howto_standard_title()}
        </h4>
        <p class="lead" data-testid="pat-intro">
          {m.connection_howto_pat_intro()}
        </p>

        <div
          class="callout callout--warning"
          data-testid="pat-token-type-callout"
        >
          <p>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html renderBold(m.connection_howto_pat_token_type_label())}
          </p>
          <p>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html renderBold(m.connection_howto_pat_token_type_warning())}
          </p>
          <p class="small">
            <a
              href="https://github.com/cli/cli/issues/6680"
              target="_blank"
              rel="noopener noreferrer"
            >
              {m.connection_howto_pat_token_type_link_label()}
            </a>
          </p>
        </div>

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

        <div class="callout" data-testid="pat-sso-callout">
          <p class="callout-title">
            {m.connection_howto_pat_sso_title()}
          </p>
          <p>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html renderBold(m.connection_howto_pat_sso_body())}
          </p>
        </div>

        <h5 class="sub-title">
          {m.connection_howto_selfhosted_token_title()}
        </h5>
        <p>{m.connection_howto_selfhosted_token_body()}</p>
        <pre><code
            >glab auth login --hostname &lt;your-gitlab-host&gt; --token &lt;your-PAT&gt; --api-protocol https</code
          ></pre>
        <p class="small">{m.connection_howto_selfhosted_token_scope_hint()}</p>

        <details class="manual-details" data-testid="pat-manual-details">
          <summary>{m.connection_howto_pat_manual_title()}</summary>
          <p class="small">{m.connection_howto_pat_manual_body()}</p>

          <div class="manual-cmd-block">
            <div class="manual-cmd-header">
              <span class="manual-cmd-label"
                >{m.connection_howto_pat_manual_insecure_label()}</span
              >
              <Button
                size="sm"
                variant="ghost"
                icon={""}
                testid="copy-template-insecure"
                onclick={() => copyTemplate(insecureTemplate, "insecure")}
              >
                {copiedKey === "insecure"
                  ? m.connection_howto_copy_template_copied()
                  : m.connection_howto_copy_template()}
              </Button>
            </div>
            <pre><code data-testid="manual-insecure-code"
                >{insecureTemplate}</code
              ></pre>
          </div>

          <div class="manual-cmd-block">
            <div class="manual-cmd-header">
              <span class="manual-cmd-label"
                >{m.connection_howto_pat_manual_secure_label()}</span
              >
              <Button
                size="sm"
                variant="ghost"
                icon={""}
                testid="copy-template-secure"
                onclick={() => copyTemplate(secureTemplate, "secure")}
              >
                {copiedKey === "secure"
                  ? m.connection_howto_copy_template_copied()
                  : m.connection_howto_copy_template()}
              </Button>
            </div>
            <pre><code data-testid="manual-secure-code"
                >{secureTemplate}</code
              ></pre>
          </div>
        </details>
      {:else if mode === "oauth"}
        <h4 class="path-title">
          {m.connection_howto_selfhosted_oauth_title()}
        </h4>
        <p>{m.connection_howto_selfhosted_description()}</p>
        <p>{m.connection_howto_selfhosted_oauth_body()}</p>
        <pre><code>glab config set client_id &lt;client_id&gt; -g --host &lt;your-gitlab-host&gt;
glab auth login --hostname &lt;your-gitlab-host&gt;</code></pre>
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

  /*
   * Callouts — lightweight highlighted blocks used for the token-type
   * warning and the SSO note. Tokens come from the existing theme
   * palette; no new colour variables.
   */
  .callout {
    margin: 10px 0;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-left-width: 3px;
    border-radius: 4px;
    background: var(--bg-primary);
  }

  .callout p {
    margin: 4px 0;
  }

  .callout .callout-title {
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 4px 0;
  }

  .callout--warning {
    border-left-color: var(--accent-blue);
  }

  .callout--warning :global(strong) {
    color: var(--text-primary);
  }

  /*
   * Collapsed manual-login commands. Each block stacks a label row
   * (title + copy button) above a <pre><code>; the <details> element
   * keeps everything out of the way until the user opts in.
   */
  .manual-details {
    margin-top: 14px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-primary);
  }

  .manual-details > summary {
    cursor: pointer;
    font-size: 12.5px;
    color: var(--text-primary);
    padding: 2px 0;
    user-select: none;
  }

  .manual-details[open] > summary {
    margin-bottom: 4px;
  }

  .manual-cmd-block {
    margin: 10px 0;
  }

  .manual-cmd-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 2px;
  }

  .manual-cmd-label {
    font-size: 11.5px;
    color: var(--text-secondary);
    font-weight: 600;
  }
</style>
