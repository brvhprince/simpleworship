<!--
  @component
  A live countdown: the message and the time left, ticking every second.
  `onend` fires once when it reaches zero.
-->
<script lang="ts">
  import type { CountdownFrame } from "@simpleworship/protocol";
  import { onMount } from "svelte";
  import { formatClock, remainingMs, now as serviceNow } from "./timing.ts";

  interface Props {
    countdown: CountdownFrame;
    onend?: () => void;
  }

  let { countdown, onend }: Props = $props();

  let now = $state(serviceNow());
  let ended = false;

  const left = $derived(remainingMs(countdown.endsAt, now));

  onMount(() => {
    const timer = setInterval(() => {
      now = serviceNow();
      if (!ended && remainingMs(countdown.endsAt, now) === 0) {
        ended = true;
        onend?.();
      }
    }, 250);
    return () => clearInterval(timer);
  });
</script>

<div class="message">{countdown.message}</div>
<div class="time">{formatClock(left)}</div>

<style>
  .message {
    font-size: 0.6em;
  }

  .time {
    font-size: 2.2em;
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
  }
</style>
