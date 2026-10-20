<!--
  @component
  The toolbar's Stream panel: the browser source for OBS, vMix or
  Streamlabs, and NDI. The stream window (for a capture card) is in Screens, and how
  each kind of content looks on the stream is in Settings › Stream.
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { dialogs } from "../lib/dialogs.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";
  import NdiSettings from "./NdiSettings.svelte";
  import StreamLinks from "./StreamLinks.svelte";
</script>

<StreamLinks />
<NdiSettings />
<p class="hint">{t("stream.windowInScreens")}</p>
<Button
  onclick={() => {
    panels.close();
    dialogs.open = "settings";
  }}
>
  {t("stream.lookSettings")}
</Button>

<style>
  .hint {
    margin: var(--sw-space-2) 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
