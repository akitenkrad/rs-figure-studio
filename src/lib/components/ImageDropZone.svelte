<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { Upload, ImagePlus, X } from 'lucide-svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';

  let {
    maxFiles = 100,
    disabled = false,
    onFilesSelected,
  }: {
    maxFiles?: number;
    disabled?: boolean;
    onFilesSelected: (filePaths: string[]) => void;
  } = $props();

  let isDragging = $state(false);
  let selectedFiles = $state<string[]>([]);
  let errorMessage = $state<string | null>(null);
  let dragCounter = $state(0);

  // --- Drag Event Handlers ---
  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    if (disabled) return;
    dragCounter += 1;
    isDragging = true;
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    dragCounter -= 1;
    if (dragCounter <= 0) {
      isDragging = false;
      dragCounter = 0;
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    dragCounter = 0;
    if (disabled || !e.dataTransfer?.files) return;

    const files = Array.from(e.dataTransfer.files);
    const validExtensions = ['.png', '.jpg', '.jpeg'];
    const maxFileSize = 10_485_760; // 10MB

    const validPaths: string[] = [];
    const errors: string[] = [];

    for (const file of files) {
      const name = file.name.toLowerCase();
      const hasValidExt = validExtensions.some((ext) => name.endsWith(ext));

      if (!hasValidExt) {
        errors.push(`${file.name}: PNG/JPEG画像のみ対応しています`);
        continue;
      }

      if (file.size > maxFileSize) {
        errors.push(`${file.name}: ファイルサイズが10MBを超えています`);
        continue;
      }

      // In Tauri webview, dropped files provide a path via the File API
      // We use the file name; actual path handling depends on Tauri's webview behavior
      validPaths.push(file.name);
    }

    if (validPaths.length + selectedFiles.length > maxFiles) {
      errorMessage = `最大${maxFiles}ファイルまで選択できます`;
      return;
    }

    if (errors.length > 0) {
      errorMessage = errors.join('\n');
    } else {
      errorMessage = null;
    }

    if (validPaths.length > 0) {
      // For web drag-drop in Tauri, use the file dialog approach instead
      // as the webview doesn't reliably provide full file paths from drops
      errorMessage = 'ファイル選択ダイアログを使用してください（クリックまたはEnterキー）';
    }
  }

  // --- File Dialog (Tauri plugin-dialog) ---
  async function openFileDialog() {
    if (disabled) return;
    errorMessage = null;

    const selected = await open({
      multiple: true,
      filters: [
        {
          name: '画像ファイル',
          extensions: ['png', 'jpg', 'jpeg'],
        },
      ],
    });

    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];

      if (paths.length + selectedFiles.length > maxFiles) {
        errorMessage = `最大${maxFiles}ファイルまで選択できます`;
        return;
      }

      // Filter duplicates
      const newPaths = paths.filter((p) => !selectedFiles.includes(p));

      if (newPaths.length > 0) {
        selectedFiles = [...selectedFiles, ...newPaths];
        onFilesSelected(newPaths);
      }
    }
  }

  function removeFile(path: string) {
    selectedFiles = selectedFiles.filter((p) => p !== path);
  }

  function clearAll() {
    selectedFiles = [];
    errorMessage = null;
  }

  function getFileName(path: string): string {
    return path.split('/').pop() ?? path.split('\\').pop() ?? path;
  }
</script>

<div class="drop-zone-wrapper">
  <!-- Drop Zone -->
  <div
    class="drop-zone"
    class:dragging={isDragging}
    class:disabled
    role="button"
    tabindex="0"
    aria-label="画像をドラッグ&ドロップまたはクリックして選択"
    ondragenter={handleDragEnter}
    ondragleave={handleDragLeave}
    ondragover={handleDragOver}
    ondrop={handleDrop}
    onclick={openFileDialog}
    onkeydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') openFileDialog();
    }}
  >
    <div class="drop-zone-content">
      {#if isDragging}
        <Upload size={48} />
        <p class="drop-text">ドロップして追加</p>
      {:else}
        <ImagePlus size={48} />
        <p class="drop-text">画像をドラッグ&ドロップ</p>
        <p class="drop-hint">またはクリックしてファイルを選択</p>
        <p class="drop-formats">PNG / JPEG -- 最大10MB / ファイル</p>
      {/if}
    </div>
  </div>

  <!-- Error Message -->
  {#if errorMessage}
    <div class="error-message">
      <p>{errorMessage}</p>
    </div>
  {/if}

  <!-- Selected Files Preview -->
  {#if selectedFiles.length > 0}
    <div class="selected-files">
      <div class="selected-header">
        <span class="selected-count">{selectedFiles.length}ファイル選択済み</span>
        <button class="btn btn-ghost btn-sm" onclick={clearAll}>
          すべてクリア
        </button>
      </div>
      <div class="file-grid">
        {#each selectedFiles as filePath}
          <div class="file-item">
            <div class="file-thumbnail checkerboard-bg">
              <img src={convertFileSrc(filePath)} alt={getFileName(filePath)} />
            </div>
            <span class="file-name" title={getFileName(filePath)}>
              {getFileName(filePath)}
            </span>
            <button
              class="file-remove"
              onclick={(e) => {
                e.stopPropagation();
                removeFile(filePath);
              }}
              aria-label="削除"
            >
              <X size={14} />
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .drop-zone-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .drop-zone {
    border: 2px dashed var(--text-muted);
    border-radius: 12px;
    padding: var(--space-12) var(--space-6);
    text-align: center;
    cursor: pointer;
    transition:
      border-color 200ms ease,
      background-color 200ms ease;
  }

  .drop-zone:hover,
  .drop-zone.dragging {
    border-color: var(--accent-primary);
    background-color: var(--bg-tertiary);
  }

  .drop-zone:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .drop-zone.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .drop-zone-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-muted);
  }

  .drop-text {
    font-size: var(--text-base);
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
  }

  .drop-hint {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .drop-formats {
    font-size: var(--text-xs);
    color: var(--text-muted);
    margin-top: var(--space-1);
  }

  .error-message {
    padding: var(--space-3);
    background: hsla(0, 70%, 55%, 0.1);
    border: 1px solid var(--accent-danger);
    border-radius: 6px;
    color: var(--accent-danger);
    font-size: var(--text-sm);
    white-space: pre-line;
  }

  .selected-files {
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: var(--space-4);
  }

  .selected-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-3);
  }

  .selected-count {
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
  }

  .btn-sm {
    height: 28px;
    padding: 0 var(--space-2);
    font-size: var(--text-xs);
  }

  .file-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    gap: var(--space-2);
  }

  .file-item {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2);
    background: var(--bg-tertiary);
    border-radius: 6px;
    border: 1px solid var(--border-default);
  }

  .file-thumbnail {
    width: 80px;
    height: 80px;
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .file-thumbnail img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
  }

  .file-name {
    font-size: var(--text-xs);
    color: var(--text-secondary);
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }

  .file-remove {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: 50%;
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0;
    transition:
      opacity 150ms ease,
      color 150ms ease;
  }

  .file-item:hover .file-remove {
    opacity: 1;
  }

  .file-remove:hover {
    color: var(--accent-danger);
  }
</style>
