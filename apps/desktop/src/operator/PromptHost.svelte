<!--
  @component
  Shows the current question from `lib/prompts.svelte.ts`, if any.
  Render it once, in the operator window.
-->
<script lang="ts">
  import { Button, Dialog } from "@simpleworship/ui";
  import { prompts } from "../lib/prompts.svelte.ts";
</script>

{#if prompts.current}
  {@const prompt = prompts.current}
  <Dialog title={prompt.title} size="small" onclose={() => prompt.answer(prompt.cancel)}>
    <p>{prompt.message}</p>

    {#snippet actions()}
      {#each prompt.buttons as button (button.id)}
        <Button variant={button.variant} onclick={() => prompt.answer(button.id)}>
          {button.label}
        </Button>
      {/each}
    {/snippet}
  </Dialog>
{/if}

<style>
  p {
    margin: 0;
    line-height: 1.5;
  }
</style>
