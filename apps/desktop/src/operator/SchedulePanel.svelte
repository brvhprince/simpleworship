<!--
  @component
  The service schedule (left column). Building, saving and running
  schedules arrives in milestone M3; until then this explains how to put
  something on screen.
-->
<script lang="ts">
  // No logic yet; schedules arrive in milestone M3.
</script>

<section class="schedule">
  <header>Schedule</header>
  <p>
    Schedules are coming soon. For now, pick a song or type some text in the library below, then
    send it live from the preview.
  </p>
</section>

<style>
  .schedule {
    grid-area: schedule;
    background: var(--sw-surface);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  header {
    padding: var(--sw-space-2) var(--sw-space-3);
    font-weight: 600;
    border-bottom: 1px solid var(--sw-border);
  }

  p {
    margin: var(--sw-space-3);
    color: var(--sw-text-muted);
    line-height: 1.5;
  }
</style>
