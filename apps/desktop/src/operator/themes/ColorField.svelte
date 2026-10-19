<!--
  @component
  A colour: a swatch that opens the system colour picker, plus a text box
  for any CSS colour (so transparency such as `rgba(0, 0, 0, 0.6)` works,
  which the system picker cannot do).
-->
<script lang="ts">
  interface Props {
    value: string;
    label: string;
    onchange: (value: string) => void;
  }

  let { value, label, onchange }: Props = $props();

  /** The picker only understands `#rrggbb`; show black for anything else. */
  const swatch = $derived(/^#[0-9a-f]{6}$/i.test(value) ? value : "#000000");
</script>

<span class="color-field">
  <input
    type="color"
    value={swatch}
    aria-label="{label} (picker)"
    oninput={(e) => onchange(e.currentTarget.value)}
  >
  <input
    class="text"
    {value}
    aria-label={label}
    spellcheck="false"
    onchange={(e) => onchange(e.currentTarget.value.trim())}
  >
</span>

<style>
  .color-field {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  input[type="color"] {
    width: 28px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--sw-border);
    background: none;
  }

  .text {
    width: 150px;
    font-family: var(--sw-font-mono);
  }
</style>
