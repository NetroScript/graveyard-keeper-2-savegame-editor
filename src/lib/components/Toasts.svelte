<script lang="ts">
  import { cubicInOut, cubicOut } from "svelte/easing";
  import { prefersReducedMotion } from "svelte/motion";
  import CheckCircle from "~icons/ph/check-circle";
  import WarningCircle from "~icons/ph/warning-circle";
  import X from "~icons/ph/x";
  import {
    dismiss,
    pauseToasts,
    resumeToasts,
    takeEvicted,
    toastState,
    toasts,
  } from "../toasts.svelte";

  let pointerInside = false;
  let focusInside = false;
  const motion = (duration: number) =>
    prefersReducedMotion.current ? 0 : duration;

  const clamp = (value: number) => Math.min(1, Math.max(0, value));

  /**
   * Scales the toast's vertical box, including its spacing, so neighbours
   * move smoothly as it grows or collapses. Toasts stay in normal flow.
   */
  function verticalBox(node: Element) {
    const style = getComputedStyle(node);
    const sizes = {
      height: parseFloat(style.height),
      "padding-top": parseFloat(style.paddingTop),
      "padding-bottom": parseFloat(style.paddingBottom),
      "margin-bottom": parseFloat(style.marginBottom),
      "border-top-width": parseFloat(style.borderTopWidth),
      "border-bottom-width": parseFloat(style.borderBottomWidth),
    };
    return (scale: number) =>
      Object.entries(sizes)
        .map(([property, size]) => `${property}:${size * scale}px;`)
        .join("");
  }

  /** Opens space for the toast, then fades it in as it rises into place. */
  function enter(node: Element) {
    const box = verticalBox(node);
    return {
      duration: motion(360),
      css: (t: number) => {
        const size = cubicOut(clamp(t / 0.55));
        const fade = cubicOut(clamp((t - 0.25) / 0.75));
        return `overflow:hidden;${box(size)}opacity:${fade};transform:translateY(${(1 - fade) * 12}px);`;
      },
    };
  }

  /**
   * Fades the toast out first, then closes its space. Dismissed and expired
   * toasts also slide right; ones pushed out by the limit fade in place.
   */
  function leave(node: Element, id: number) {
    const evicted = takeEvicted(id);
    const box = verticalBox(node);
    return {
      duration: motion(evicted ? 520 : 460),
      css: (t: number) => {
        const fade = cubicOut(clamp((t - 0.45) / 0.55));
        const size = cubicInOut(clamp(t / 0.5));
        const shift = evicted ? 0 : (1 - fade) * 48;
        return `overflow:hidden;${box(size)}opacity:${fade};transform:translateX(${shift}px);`;
      },
    };
  }
  function update() {
    if (pointerInside || focusInside) pauseToasts();
    else resumeToasts();
  }
  $effect(() => {
    // A dismissed toast under the pointer never reports pointerleave.
    if (toasts.length === 0) {
      pointerInside = focusInside = false;
      resumeToasts();
    }
  });
</script>

<section
  class="toasts"
  class:paused={toastState.paused}
  aria-label="Notifications"
  onpointerenter={() => {
    pointerInside = true;
    update();
  }}
  onpointerleave={() => {
    pointerInside = false;
    update();
  }}
  onfocusin={() => {
    focusInside = true;
    update();
  }}
  onfocusout={(event) => {
    focusInside = event.currentTarget.contains(
      event.relatedTarget as Node | null,
    );
    update();
  }}
>
  {#each toasts as toast (toast.id)}<div
      class={`toast ${toast.kind}`}
      role={toast.kind === "error" ? "alert" : "status"}
      in:enter
      out:leave={toast.id}
    >
      {#if toast.kind === "error"}<WarningCircle />{:else}<CheckCircle />{/if}
      <p>{toast.message}</p>
      <button
        class="dismiss"
        aria-label="Dismiss notification"
        onclick={() => dismiss(toast.id)}><X /></button
      >
      <span
        class="remaining"
        style={`animation-duration:${toast.duration}ms`}
        aria-hidden="true"
      ></span>
    </div>{/each}
</section>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    /* Each toast carries its 8 px spacing so a collapsing toast removes it too. */
    bottom: 8px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    width: min(420px, calc(100vw - 32px));
    pointer-events: none;
  }
  .toast {
    position: relative;
    display: grid;
    grid-template-columns: 19px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
    padding: 8px 8px 8px 12px;
    overflow: hidden;
    font-size: 14px;
    border: 1px solid;
    pointer-events: auto;
  }
  .toast.error {
    color: #e6a798;
    background: #512d2c;
    border-color: #85443a;
  }
  .toast.success {
    color: #cad7b5;
    background: #293329;
    border-color: #586c4e;
  }
  .toast > :global(svg) {
    display: block;
    width: 19px;
    height: 19px;
  }
  p {
    margin: 0;
    line-height: 1.4;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
  .dismiss {
    display: grid;
    place-items: center;
    min-height: 0;
    padding: 5px;
    color: inherit;
    background: transparent;
    border-color: transparent;
  }
  .dismiss :global(svg) {
    width: 14px;
    height: 14px;
  }
  .remaining {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 2px;
    background: currentColor;
    opacity: 0.5;
    transform-origin: left;
    animation: remaining linear forwards;
  }
  .paused .remaining {
    animation-play-state: paused;
  }
  @keyframes remaining {
    to {
      transform: scaleX(0);
    }
  }
</style>
