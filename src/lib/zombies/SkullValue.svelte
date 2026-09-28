<script lang="ts">
  import GameIcon from "../components/GameIcon.svelte";
  let {
    red = 0,
    white = 0,
    compact = false,
  }: {
    red?: number | string;
    white?: number | string;
    compact?: boolean;
  } = $props();
</script>

<span
  class="skulls"
  class:compact
  aria-label={`${red} red skulls, ${white} white skulls`}
>
  {#if Number(red) !== 0}<span><GameIcon name="rskull" />{red}</span>{/if}
  {#if Number(white) !== 0 || Number(red) === 0}<span
      ><GameIcon name="skull" />{white}</span
    >{/if}
</span>

<style>
  .skulls,
  .skulls > span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .skulls {
    gap: 8px;
    font-variant-numeric: tabular-nums;
  }
  .compact {
    gap: 4px;
    font-size: 11px;
  }
  .compact :global(.game-icon) {
    width: 18px;
    height: 18px;
    flex-basis: 18px;
  }
</style>
