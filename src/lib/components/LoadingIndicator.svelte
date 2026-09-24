<script lang="ts">
  let {
    label = "Loading…",
    compact = false,
  }: { label?: string; compact?: boolean } = $props();
</script>

<div class:compact class="loading-stage" role="status" aria-live="polite">
  <span>{label}</span>
  <span class="loading-track" aria-hidden="true"><i></i></span>
</div>

<style>
  .loading-stage {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 12px;
    min-height: 240px;
    width: 100%;
    color: var(--muted);
    font-size: 12px;
  }
  .loading-stage.compact {
    min-height: 100px;
  }
  .loading-track {
    position: relative;
    display: block;
    width: min(280px, 70%);
    height: 6px;
    overflow: hidden;
    background: #171920;
    border: 1px solid #4e5059;
  }
  .loading-track i {
    position: absolute;
    inset: 0 auto 0 0;
    width: 38%;
    background: #c8952c;
    transform: translate3d(-100%, 0, 0);
    will-change: transform;
    animation: loading-slide 1.05s ease-in-out infinite;
  }
  @keyframes loading-slide {
    0% {
      transform: translate3d(-100%, 0, 0);
    }
    55% {
      transform: translate3d(264%, 0, 0);
    }
    100% {
      transform: translate3d(264%, 0, 0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .loading-track i {
      animation-duration: 1.8s;
      animation-timing-function: linear;
    }
  }
</style>
