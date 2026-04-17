<!--
  Connection how-to — collapsible help block explaining how to
  authenticate BeardGit against gitlab.com / github.com, and against
  self-hosted GitLab (OAuth client_id path OR the token fallback).

  Shown in Settings > Connection above the existing Token Auth /
  CLI Auth sections so users who hit auth errors have a one-stop
  reference without leaving the app.
-->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";

  let open = $state(false);

  function toggle() {
    open = !open;
  }
</script>

<section class="howto">
  <button
    class="howto-header"
    type="button"
    aria-expanded={open}
    onclick={toggle}
  >
    <span class="chevron nf" class:open>&#xF105;</span>
    <span class="howto-title">{m.connection_howto_title()}</span>
    <span class="howto-hint">{m.connection_howto_hint()}</span>
  </button>

  {#if open}
    <div class="howto-body">
      <p class="lead">{m.connection_howto_lead()}</p>

      <h4 class="path-title">{m.connection_howto_standard_title()}</h4>
      <p>{m.connection_howto_standard_description()}</p>
      <ul>
        <li>
          <strong>{m.connection_howto_standard_token_label()}</strong>
          {m.connection_howto_standard_token_body()}
          <ul>
            <li>
              GitHub: <a href="https://github.com/settings/tokens" target="_blank" rel="noopener noreferrer">github.com/settings/tokens</a>
              — {m.connection_howto_scopes_github()}
            </li>
            <li>
              GitLab: <a href="https://gitlab.com/-/user_settings/personal_access_tokens" target="_blank" rel="noopener noreferrer">gitlab.com/-/user_settings/personal_access_tokens</a>
              — {m.connection_howto_scopes_gitlab()}
            </li>
          </ul>
        </li>
        <li>
          <strong>{m.connection_howto_standard_cli_label()}</strong>
          {m.connection_howto_standard_cli_body()}
          <pre><code>gh auth login
glab auth login</code></pre>
        </li>
      </ul>

      <h4 class="path-title">{m.connection_howto_selfhosted_title()}</h4>
      <p>{m.connection_howto_selfhosted_description()}</p>

      <h5 class="sub-title">{m.connection_howto_selfhosted_oauth_title()}</h5>
      <p>{m.connection_howto_selfhosted_oauth_body()}</p>
      <pre><code>glab config set client_id &lt;client_id&gt; -g --host &lt;your-gitlab-host&gt;
glab auth login --hostname &lt;your-gitlab-host&gt;</code></pre>

      <h5 class="sub-title">{m.connection_howto_selfhosted_token_title()}</h5>
      <p>{m.connection_howto_selfhosted_token_body()}</p>
      <pre><code>glab auth login --hostname &lt;your-gitlab-host&gt; --token &lt;your-PAT&gt; --api-protocol https</code></pre>
      <p class="small">{m.connection_howto_selfhosted_token_scope_hint()}</p>

      <h4 class="path-title">{m.connection_howto_troubleshoot_title()}</h4>
      <ul>
        <li>
          <strong>{m.connection_howto_troubleshoot_multi_config_label()}</strong>
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
</section>

<style>
  .howto {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-secondary);
    margin-bottom: 20px;
    overflow: hidden;
  }

  .howto-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
    text-align: left;
  }

  .howto-header:hover {
    background: var(--bg-hover);
  }

  .chevron {
    display: inline-block;
    transition: transform 0.15s ease;
    font-size: 12px;
    color: var(--text-secondary);
    width: 12px;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .howto-title {
    font-weight: 600;
    color: var(--text-primary);
  }

  .howto-hint {
    color: var(--text-secondary);
    font-size: 12px;
    margin-left: auto;
  }

  .howto-body {
    padding: 8px 20px 20px 20px;
    border-top: 1px solid var(--border);
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-primary);
  }

  .lead {
    color: var(--text-secondary);
    margin: 12px 0 18px 0;
  }

  .path-title {
    margin: 20px 0 8px 0;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent-blue);
  }

  .sub-title {
    margin: 16px 0 6px 0;
    font-size: 12.5px;
    color: var(--text-primary);
  }

  ul {
    margin: 6px 0;
    padding-left: 18px;
  }

  ul ul {
    margin: 4px 0 8px 0;
  }

  li {
    margin: 6px 0;
  }

  pre {
    margin: 8px 0;
    padding: 10px 12px;
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
    margin: 6px 0 0 0;
  }
</style>
