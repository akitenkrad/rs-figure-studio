<script lang="ts">
  import { Play, Pause, SkipForward, ZoomIn, ZoomOut, RotateCcw } from 'lucide-svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { AnimationDef } from '$lib/types';

  let {
    spritesheetPath,
    tileWidth,
    tileHeight,
    directions,
    animations,
  }: {
    spritesheetPath: string;
    tileWidth: number;
    tileHeight: number;
    directions: string[];
    animations: AnimationDef[];
  } = $props();

  // --- Internal State ---
  let canvas: HTMLCanvasElement | undefined = $state(undefined);
  let playing = $state(true);
  let currentDirectionIndex = $state(0);
  let currentAnimationIndex = $state(0);
  let currentFrame = $state(0);
  let zoom = $state(4);
  let playbackSpeed = $state(1);
  let showGrid = $state(false);
  let spritesheetImage: HTMLImageElement | null = $state(null);
  let animationFrameId: number | null = null;
  let lastFrameTime = 0;

  // --- Derived ---
  const activeDirection = $derived(directions[currentDirectionIndex] ?? 'down');
  const activeAnimation = $derived(animations[currentAnimationIndex]);
  const canvasWidth = $derived(tileWidth * zoom);
  const canvasHeight = $derived(tileHeight * zoom);

  // Calculate column offset for the current animation
  // Animations are concatenated horizontally: idle(3) + walk(3) + attack(3) + ...
  const animationStartCol = $derived(
    (() => {
      let col = 0;
      for (let i = 0; i < currentAnimationIndex; i++) {
        col += animations[i].frame_count;
      }
      return col;
    })(),
  );

  const speedLabel = $derived(
    playbackSpeed === 0.5 ? '0.5x' : playbackSpeed === 1 ? '1x' : '2x',
  );

  // --- Load spritesheet image ---
  $effect(() => {
    if (!spritesheetPath) return;
    const img = new Image();
    img.onload = () => {
      spritesheetImage = img;
      draw();
    };
    img.onerror = () => {
      spritesheetImage = null;
    };
    img.src = convertFileSrc(spritesheetPath);
    return () => {
      img.onload = null;
      img.onerror = null;
    };
  });

  // --- Animation loop ---
  $effect(() => {
    if (playing && spritesheetImage && activeAnimation) {
      startAnimationLoop();
    }
    return () => {
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
      }
    };
  });

  // Redraw when zoom, direction, animation changes (when not playing)
  $effect(() => {
    // Access reactive dependencies
    void zoom;
    void currentDirectionIndex;
    void currentAnimationIndex;
    void showGrid;
    if (!playing && spritesheetImage) {
      draw();
    }
  });

  function startAnimationLoop() {
    lastFrameTime = performance.now();
    function loop(time: number) {
      if (!activeAnimation) return;
      const frameDuration = activeAnimation.frame_duration_ms / playbackSpeed;
      const elapsed = time - lastFrameTime;

      if (elapsed >= frameDuration) {
        currentFrame = (currentFrame + 1) % activeAnimation.frame_count;
        lastFrameTime = time;
        draw();
      }
      animationFrameId = requestAnimationFrame(loop);
    }
    animationFrameId = requestAnimationFrame(loop);
  }

  // --- Canvas Drawing ---
  function draw() {
    if (!canvas || !spritesheetImage || !activeAnimation) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Checkerboard background
    drawCheckerboard(ctx);

    // Sprite drawing
    const srcX = (animationStartCol + currentFrame) * tileWidth;
    const srcY = currentDirectionIndex * tileHeight;

    ctx.imageSmoothingEnabled = false; // Pixel art
    ctx.drawImage(
      spritesheetImage,
      srcX,
      srcY,
      tileWidth,
      tileHeight,
      0,
      0,
      canvasWidth,
      canvasHeight,
    );

    // Grid lines (optional)
    if (showGrid) {
      drawGrid(ctx);
    }
  }

  function drawCheckerboard(ctx: CanvasRenderingContext2D) {
    const size = 8 * zoom;
    for (let y = 0; y < canvasHeight; y += size) {
      for (let x = 0; x < canvasWidth; x += size) {
        const isLight = (Math.floor(x / size) + Math.floor(y / size)) % 2 === 0;
        ctx.fillStyle = isLight ? '#3a3a3a' : '#2a2a2a';
        ctx.fillRect(x, y, size, size);
      }
    }
  }

  function drawGrid(ctx: CanvasRenderingContext2D) {
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
    ctx.lineWidth = 1;
    const gridSize = zoom;
    for (let x = 0; x <= canvasWidth; x += gridSize) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, canvasHeight);
      ctx.stroke();
    }
    for (let y = 0; y <= canvasHeight; y += gridSize) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(canvasWidth, y);
      ctx.stroke();
    }
  }

  // --- Controls ---
  function togglePlayback() {
    playing = !playing;
    if (!playing) draw();
  }

  function stepForward() {
    if (playing) playing = false;
    if (!activeAnimation) return;
    currentFrame = (currentFrame + 1) % activeAnimation.frame_count;
    draw();
  }

  function cycleSpeed() {
    if (playbackSpeed === 0.5) playbackSpeed = 1;
    else if (playbackSpeed === 1) playbackSpeed = 2;
    else playbackSpeed = 0.5;
  }

  function zoomIn() {
    if (zoom < 8) {
      zoom += 1;
      draw();
    }
  }

  function zoomOut() {
    if (zoom > 1) {
      zoom -= 1;
      draw();
    }
  }

  function resetView() {
    zoom = 4;
    currentFrame = 0;
    playbackSpeed = 1;
    playing = true;
  }

  function changeDirection(index: number) {
    currentDirectionIndex = index;
    currentFrame = 0;
    draw();
  }

  function changeAnimation(index: number) {
    currentAnimationIndex = index;
    currentFrame = 0;
    draw();
  }

  const directionLabels: Record<string, string> = {
    down: '下',
    left: '左',
    right: '右',
    up: '上',
  };

  function getDirectionLabel(dir: string): string {
    return directionLabels[dir] ?? dir;
  }
</script>

<div class="sprite-preview">
  <!-- Control Bar -->
  <div class="controls">
    <!-- Direction & Animation Selectors -->
    <div class="control-group selectors">
      <div class="selector">
        <span class="selector-label">方向</span>
        <div class="selector-buttons" role="group" aria-label="方向">
          {#each directions as dir, i}
            <button
              class="selector-btn"
              class:active={currentDirectionIndex === i}
              onclick={() => changeDirection(i)}
            >
              {getDirectionLabel(dir)}
            </button>
          {/each}
        </div>
      </div>

      <div class="selector">
        <span class="selector-label">アニメーション</span>
        <div class="selector-buttons" role="group" aria-label="アニメーション">
          {#each animations as anim, i}
            <button
              class="selector-btn"
              class:active={currentAnimationIndex === i}
              onclick={() => changeAnimation(i)}
            >
              {anim.name}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Playback Controls -->
    <div class="control-group playback">
      <button
        class="ctrl-btn"
        onclick={togglePlayback}
        aria-label={playing ? '停止' : '再生'}
        title={playing ? '停止' : '再生'}
      >
        {#if playing}<Pause size={16} />{:else}<Play size={16} />{/if}
      </button>
      <button
        class="ctrl-btn"
        onclick={stepForward}
        aria-label="次のフレーム"
        title="次のフレーム"
      >
        <SkipForward size={16} />
      </button>
      <button
        class="ctrl-btn speed-btn"
        onclick={cycleSpeed}
        aria-label="速度変更"
        title="速度変更"
      >
        {speedLabel}
      </button>
    </div>

    <!-- Zoom Controls -->
    <div class="control-group zoom">
      <button class="ctrl-btn" onclick={zoomOut} aria-label="縮小" title="縮小">
        <ZoomOut size={16} />
      </button>
      <span class="zoom-label">{zoom}x</span>
      <button class="ctrl-btn" onclick={zoomIn} aria-label="拡大" title="拡大">
        <ZoomIn size={16} />
      </button>
      <button class="ctrl-btn" onclick={resetView} aria-label="リセット" title="リセット">
        <RotateCcw size={16} />
      </button>
    </div>

    <!-- Grid Toggle -->
    <label class="control-checkbox">
      <input type="checkbox" bind:checked={showGrid} />
      グリッド
    </label>
  </div>

  <!-- Canvas -->
  <div class="canvas-container checkerboard-bg">
    <canvas
      bind:this={canvas}
      width={canvasWidth}
      height={canvasHeight}
    ></canvas>
  </div>

  <!-- Frame Info -->
  {#if activeAnimation}
    <div class="frame-info">
      <span class="frame-counter">
        フレーム: {currentFrame + 1} / {activeAnimation.frame_count}
      </span>
      <span class="frame-timing">
        {activeAnimation.frame_duration_ms}ms/frame
      </span>
      <span class="frame-fps">
        {Math.round(1000 / activeAnimation.frame_duration_ms)} FPS
      </span>
      <span class="frame-direction">
        {getDirectionLabel(activeDirection)} / {activeAnimation.name}
      </span>
    </div>
  {/if}
</div>

<style>
  .sprite-preview {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .control-group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .selectors {
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .selector {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .selector-label {
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
  }

  .selector-buttons {
    display: flex;
    gap: 2px;
    background: var(--bg-primary);
    border-radius: 6px;
    padding: 2px;
  }

  .selector-btn {
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition:
      background-color 150ms ease,
      color 150ms ease;
    font-family: var(--font-sans);
  }

  .selector-btn:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .selector-btn.active {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: var(--bg-primary);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    transition:
      background-color 150ms ease,
      color 150ms ease;
  }

  .ctrl-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .speed-btn {
    width: auto;
    padding: 0 var(--space-2);
    font-size: var(--text-xs);
    font-weight: var(--font-weight-semibold);
    font-family: var(--font-mono);
  }

  .zoom-label {
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-muted);
    font-family: var(--font-mono);
    min-width: 24px;
    text-align: center;
  }

  .control-checkbox {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
  }

  .control-checkbox input[type='checkbox'] {
    accent-color: var(--accent-primary);
  }

  .canvas-container {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    overflow: auto;
    min-height: 200px;
  }

  .canvas-container canvas {
    image-rendering: pixelated;
  }

  .frame-info {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    color: var(--text-muted);
  }

  .frame-counter {
    color: var(--text-secondary);
  }
</style>
