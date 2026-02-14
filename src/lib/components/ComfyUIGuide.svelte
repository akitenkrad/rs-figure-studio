<script lang="ts">
  import { Check, X, Loader2, Terminal, Download, Wifi, Puzzle, Copy } from 'lucide-svelte';

  type GuideTab = 'install' | 'custom-nodes' | 'launch' | 'test';

  let {
    endpoint,
    connected,
    onEndpointChange,
    onConnectionTest,
  }: {
    endpoint: string;
    connected: boolean;
    onEndpointChange: (endpoint: string) => void;
    onConnectionTest: () => Promise<boolean>;
  } = $props();

  let activeTab = $state<GuideTab>('install');
  let testing = $state(false);
  let testResult = $state<boolean | null>(null);

  // OS detection via navigator, used as default for dropdown
  const detectedOS = (() => {
    if (typeof navigator === 'undefined') return 'linux';
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('mac') || ua.includes('darwin')) return 'macos';
    if (ua.includes('win')) return 'windows';
    return 'linux';
  })();

  let selectedOS = $state(detectedOS);

  // OS-specific install commands
  const installCommands: Record<string, string> = {
    macos: `# uvをインストール
curl -LsSf https://astral.sh/uv/install.sh | sh

# ComfyUIをクローン
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI

# uvで仮想環境を作成して依存関係をインストール
uv venv
source .venv/bin/activate
uv pip install -r requirements.txt`,
    windows: `# uvをインストール (PowerShell)
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"

# ComfyUIをクローン
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI

# uvで仮想環境を作成して依存関係をインストール
uv venv
.venv\\Scripts\\activate
uv pip install -r requirements.txt

# またはComfyUI Desktop版をダウンロード
# https://github.com/comfyanonymous/ComfyUI/releases`,
    linux: `# システムパッケージをインストール (Ubuntu/Debian)
sudo apt update
sudo apt install -y python3 git

# NVIDIA GPUドライバとCUDAをインストール (推奨)
# https://developer.nvidia.com/cuda-downloads からOS/バージョンを選択
# 例: Ubuntu 22.04/24.04
sudo apt install -y nvidia-driver-550
sudo apt install -y nvidia-cuda-toolkit

# uvをインストール
curl -LsSf https://astral.sh/uv/install.sh | sh

# ComfyUIをクローン
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI

# uvで仮想環境を作成
uv venv
source .venv/bin/activate

# PyTorch (CUDA版) をインストール
uv pip install torch torchvision torchaudio \\
  --extra-index-url https://download.pytorch.org/whl/cu121

# 依存関係をインストール
uv pip install -r requirements.txt

# AMD GPU (ROCm) の場合は以下を代わりに使用:
# uv pip install torch torchvision torchaudio \\
#   --extra-index-url https://download.pytorch.org/whl/rocm6.0`,
  };

  // OS-specific launch commands
  const launchCommands: Record<string, string> = {
    macos: `cd ComfyUI
source .venv/bin/activate
python main.py --listen 127.0.0.1 --port 8188`,
    windows: `cd ComfyUI
.venv\\Scripts\\activate
python main.py --listen 127.0.0.1 --port 8188`,
    linux: `cd ComfyUI
source .venv/bin/activate
python3 main.py --listen 127.0.0.1 --port 8188`,
  };

  const osLabel: Record<string, string> = {
    macos: 'macOS',
    windows: 'Windows',
    linux: 'Linux',
  };

  // All-in-one install script
  const allInOneScript = `#!/bin/bash
set -e

# ComfyUI のルートディレクトリで実行してください
# 例: cd ~/ComfyUI

COMFYUI_DIR="\${1:-.}"
cd "$COMFYUI_DIR"

echo "=== カスタムノードをインストール ==="

cd custom_nodes

# ComfyUI Manager
if [ ! -d "ComfyUI-Manager" ]; then
  echo "[1/3] ComfyUI Manager をインストール中..."
  git clone https://github.com/ltdrdata/ComfyUI-Manager.git
else
  echo "[1/3] ComfyUI Manager: 既にインストール済み"
fi

# ComfyUI_IPAdapter_plus
if [ ! -d "ComfyUI_IPAdapter_plus" ]; then
  echo "[2/3] ComfyUI_IPAdapter_plus をインストール中..."
  git clone https://github.com/cubiq/ComfyUI_IPAdapter_plus.git
else
  echo "[2/3] ComfyUI_IPAdapter_plus: 既にインストール済み"
fi

# comfyui_controlnet_aux
if [ ! -d "comfyui_controlnet_aux" ]; then
  echo "[3/3] comfyui_controlnet_aux をインストール中..."
  git clone https://github.com/Fannovel16/comfyui_controlnet_aux.git
  cd comfyui_controlnet_aux
  uv pip install -r requirements.txt
  cd ..
else
  echo "[3/3] comfyui_controlnet_aux: 既にインストール済み"
fi

cd "$COMFYUI_DIR"

echo ""
echo "=== モデルファイルをダウンロード ==="

mkdir -p models/ipadapter models/clip_vision models/checkpoints

# IP-Adapter Plus (SDXL)
if [ ! -f "models/ipadapter/ip-adapter-plus_sdxl_vit-h.safetensors" ]; then
  echo "IP-Adapter Plus モデルをダウンロード中..."
  curl -L -o models/ipadapter/ip-adapter-plus_sdxl_vit-h.safetensors \\
    https://huggingface.co/h94/IP-Adapter/resolve/main/sdxl_models/ip-adapter-plus_sdxl_vit-h.safetensors
else
  echo "IP-Adapter Plus モデル: 既にダウンロード済み"
fi

# CLIP Vision
if [ ! -f "models/clip_vision/CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors" ]; then
  echo "CLIP Vision モデルをダウンロード中..."
  curl -L -o models/clip_vision/CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors \\
    https://huggingface.co/h94/IP-Adapter/resolve/main/models/image_encoder/model.safetensors
else
  echo "CLIP Vision モデル: 既にダウンロード済み"
fi

# SDXL Base checkpoint
if [ ! -f "models/checkpoints/sd_xl_base_1.0.safetensors" ]; then
  echo "SDXL Base モデルをダウンロード中 (約6.5GB)..."
  curl -L -o models/checkpoints/sd_xl_base_1.0.safetensors \\
    https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0/resolve/main/sd_xl_base_1.0.safetensors
else
  echo "SDXL Base モデル: 既にダウンロード済み"
fi

echo ""
echo "=== セットアップ完了 ==="
echo "ComfyUI を再起動してください．"`;

  let copied = $state(false);

  async function copyScript() {
    try {
      await navigator.clipboard.writeText(allInOneScript);
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      // fallback
    }
  }

  // Connection test handler
  async function runConnectionTest() {
    testing = true;
    testResult = null;
    try {
      testResult = await onConnectionTest();
    } catch {
      testResult = false;
    } finally {
      testing = false;
    }
  }
</script>

<div class="comfyui-guide">
  <!-- Tab Navigation -->
  <div class="tab-nav" role="tablist">
    <button
      class="tab-button"
      role="tab"
      aria-selected={activeTab === 'install'}
      class:active={activeTab === 'install'}
      onclick={() => (activeTab = 'install')}
    >
      <Download size={16} />
      インストール
    </button>
    <button
      class="tab-button"
      role="tab"
      aria-selected={activeTab === 'custom-nodes'}
      class:active={activeTab === 'custom-nodes'}
      onclick={() => (activeTab = 'custom-nodes')}
    >
      <Puzzle size={16} />
      カスタムノード
    </button>
    <button
      class="tab-button"
      role="tab"
      aria-selected={activeTab === 'launch'}
      class:active={activeTab === 'launch'}
      onclick={() => (activeTab = 'launch')}
    >
      <Terminal size={16} />
      起動コマンド
    </button>
    <button
      class="tab-button"
      role="tab"
      aria-selected={activeTab === 'test'}
      class:active={activeTab === 'test'}
      onclick={() => (activeTab = 'test')}
    >
      <Wifi size={16} />
      接続テスト
      {#if connected}
        <span class="connected-dot"></span>
      {/if}
    </button>
  </div>

  <!-- Tab Content -->
  <div class="tab-content" role="tabpanel">
    {#if activeTab === 'install'}
      <div class="tab-panel">
        <h4>ComfyUI インストール手順</h4>
        <div class="os-selector">
          <label class="label" for="os-select">OS を選択:</label>
          <select
            id="os-select"
            class="input os-select"
            bind:value={selectedOS}
          >
            {#each Object.entries(osLabel) as [key, label]}
              <option value={key}>{label}</option>
            {/each}
          </select>
        </div>

        <div class="code-block">
          <pre><code>{installCommands[selectedOS]}</code></pre>
        </div>

        <div class="note">
          <p>
            AI キャラクター生成機能を使用するには，追加のカスタムノードが必要です．
            「カスタムノード」タブを参照してください．
          </p>
        </div>
      </div>
    {:else if activeTab === 'custom-nodes'}
      <div class="tab-panel">
        <h4>カスタムノードのインストール</h4>

        <div class="note">
          <p>
            AI キャラクター生成（方向展開・アニメーション展開）には以下のカスタムノードが必要です．
            ComfyUI の仮想環境を有効化した状態で実行してください．
          </p>
        </div>

        <!-- All-in-one script -->
        <div class="all-in-one">
          <div class="all-in-one-header">
            <h5 class="section-title">一括インストールスクリプト</h5>
            <button class="btn btn-ghost btn-sm copy-btn" onclick={copyScript}>
              {#if copied}
                <Check size={14} />
                コピー済み
              {:else}
                <Copy size={14} />
                コピー
              {/if}
            </button>
          </div>
          <p class="section-desc">
            以下のスクリプトでカスタムノードとモデルを一括インストールできます．
            ComfyUI のルートディレクトリで実行してください．
          </p>
          <div class="code-block code-block-scroll">
            <pre><code>{allInOneScript}</code></pre>
          </div>
          <div class="usage-hint">
            <code>bash setup_figurine_studio.sh /path/to/ComfyUI</code>
          </div>
        </div>

        <hr class="divider" />

        <h5 class="section-title">個別インストール手順</h5>

        <h5 class="section-title">1. ComfyUI Manager（推奨）</h5>
        <p class="section-desc">カスタムノードの管理を簡単にするツールです．</p>
        <div class="code-block">
          <pre><code>cd ComfyUI/custom_nodes
git clone https://github.com/ltdrdata/ComfyUI-Manager.git</code></pre>
        </div>

        <h5 class="section-title">2. ComfyUI_IPAdapter_plus（必須）</h5>
        <p class="section-desc">IP-Adapter によるキャラクターの一貫性維持に使用します．</p>
        <div class="code-block">
          <pre><code>cd ComfyUI/custom_nodes
git clone https://github.com/cubiq/ComfyUI_IPAdapter_plus.git

# IP-Adapter モデルをダウンロード
cd ComfyUI/models
mkdir -p ipadapter clip_vision

# IP-Adapter Plus モデル (SDXL用)
curl -L -o ipadapter/ip-adapter-plus_sdxl_vit-h.safetensors \
  https://huggingface.co/h94/IP-Adapter/resolve/main/sdxl_models/ip-adapter-plus_sdxl_vit-h.safetensors

# CLIP Vision モデル
curl -L -o clip_vision/CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors \
  https://huggingface.co/h94/IP-Adapter/resolve/main/models/image_encoder/model.safetensors</code></pre>
        </div>

        <h5 class="section-title">3. comfyui_controlnet_aux（推奨）</h5>
        <p class="section-desc">ControlNet の前処理ノード（OpenPose / DWPose）を提供します．</p>
        <div class="code-block">
          <pre><code>cd ComfyUI/custom_nodes
git clone https://github.com/Fannovel16/comfyui_controlnet_aux.git
cd comfyui_controlnet_aux
uv pip install -r requirements.txt</code></pre>
        </div>

        <h5 class="section-title">4. ベースモデル</h5>
        <p class="section-desc">チェックポイントモデルを配置します．</p>
        <div class="code-block">
          <pre><code># SDXL Base モデルを models/checkpoints/ に配置
# https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0
curl -L -o ComfyUI/models/checkpoints/sd_xl_base_1.0.safetensors \
  https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0/resolve/main/sd_xl_base_1.0.safetensors</code></pre>
        </div>

        <div class="note">
          <p>
            インストール後，ComfyUI を再起動してください．
            ComfyUI Manager を使用すると，Web UI 上から不足ノードの検索・インストールも可能です．
          </p>
        </div>
      </div>
    {:else if activeTab === 'launch'}
      <div class="tab-panel">
        <h4>ComfyUI 起動コマンド</h4>
        <div class="os-selector">
          <label class="label" for="os-select-launch">OS を選択:</label>
          <select
            id="os-select-launch"
            class="input os-select"
            bind:value={selectedOS}
          >
            {#each Object.entries(osLabel) as [key, label]}
              <option value={key}>{label}</option>
            {/each}
          </select>
        </div>

        <div class="code-block">
          <pre><code>{launchCommands[selectedOS]}</code></pre>
        </div>

        <div class="note">
          <p>
            起動後，ブラウザで <code>http://127.0.0.1:8188</code> にアクセスして
            ComfyUI Web UI が表示されることを確認してください．
          </p>
        </div>

        <div class="tips">
          <h5>ヒント</h5>
          <ul>
            <li><code>--listen 0.0.0.0</code> で外部からのアクセスを許可できます</li>
            <li><code>--port 8188</code> でポート番号を変更できます</li>
            <li><code>--enable-cors-header</code> でCORSを有効化できます</li>
            <li>GPU メモリが不足する場合は <code>--lowvram</code> オプションを追加してください</li>
          </ul>
        </div>
      </div>
    {:else if activeTab === 'test'}
      <div class="tab-panel">
        <h4>接続テスト</h4>

        <div class="endpoint-form">
          <div class="form-group">
            <label class="label" for="comfyui-endpoint">エンドポイント URL</label>
            <div class="input-row">
              <input
                id="comfyui-endpoint"
                type="url"
                class="input"
                value={endpoint}
                oninput={(e) => onEndpointChange(e.currentTarget.value)}
                placeholder="http://127.0.0.1:8188"
              />
              <button
                class="btn btn-primary"
                onclick={runConnectionTest}
                disabled={testing || !endpoint}
              >
                {#if testing}
                  <Loader2 size={16} class="spin" />
                  テスト中...
                {:else}
                  <Wifi size={16} />
                  接続テスト
                {/if}
              </button>
            </div>
          </div>
        </div>

        <!-- Connection Status -->
        <div class="connection-status">
          {#if testing}
            <div class="status-indicator testing">
              <Loader2 size={20} class="spin" />
              <span>ComfyUI に接続しています...</span>
            </div>
          {:else if testResult === true}
            <div class="status-indicator success">
              <Check size={20} />
              <div class="status-text">
                <strong>接続成功</strong>
                <span>ComfyUI が正常に動作しています</span>
              </div>
            </div>
          {:else if testResult === false}
            <div class="status-indicator failure">
              <X size={20} />
              <div class="status-text">
                <strong>接続失敗</strong>
                <span>ComfyUI が起動しているか確認してください</span>
              </div>
            </div>
          {:else if connected}
            <div class="status-indicator success">
              <Check size={20} />
              <div class="status-text">
                <strong>接続済み</strong>
                <span>ComfyUI と接続されています</span>
              </div>
            </div>
          {:else}
            <div class="status-indicator idle">
              <Wifi size={20} />
              <span>「接続テスト」ボタンを押して接続を確認してください</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .comfyui-guide {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-default);
    border-radius: 12px;
    overflow: hidden;
    background: var(--bg-secondary);
  }

  /* Tab Navigation */
  .tab-nav {
    display: flex;
    border-bottom: 1px solid var(--border-default);
    background: var(--bg-tertiary);
  }

  .tab-button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    font-family: var(--font-sans);
    cursor: pointer;
    transition:
      color 150ms ease,
      border-color 150ms ease,
      background-color 150ms ease;
    position: relative;
  }

  .tab-button:hover {
    color: var(--text-secondary);
    background: var(--bg-elevated);
  }

  .tab-button.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }

  .connected-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-success);
    display: inline-block;
  }

  /* Tab Content */
  .tab-content {
    padding: var(--space-6);
  }

  .tab-panel h4 {
    margin-bottom: var(--space-3);
  }

  .os-selector {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }

  .os-selector .label {
    margin-bottom: 0;
    white-space: nowrap;
    font-size: var(--text-sm);
  }

  .os-select {
    width: auto;
    min-width: 140px;
  }

  /* Code Block */
  .code-block {
    background: var(--bg-primary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: var(--space-4);
    margin-bottom: var(--space-4);
    overflow-x: auto;
  }

  .code-block pre {
    margin: 0;
  }

  .code-block code {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text-primary);
    line-height: var(--leading-relaxed);
    white-space: pre;
  }

  /* Note */
  .note {
    padding: var(--space-4);
    background: var(--bg-tertiary);
    border-radius: 8px;
    border-left: 3px solid var(--accent-info);
    margin-bottom: var(--space-4);
  }

  .note p {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: var(--leading-relaxed);
  }

  .note code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    background: var(--bg-primary);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .section-title {
    font-size: var(--text-sm);
    font-weight: var(--font-weight-semibold);
    margin-bottom: var(--space-1);
    margin-top: var(--space-4);
  }

  .section-title:first-of-type {
    margin-top: 0;
  }

  .section-desc {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-2);
  }

  .all-in-one {
    margin-bottom: var(--space-4);
  }

  .all-in-one-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-1);
  }

  .all-in-one-header .section-title {
    margin: 0;
  }

  .copy-btn {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
    min-width: 90px;
    justify-content: center;
  }

  .code-block-scroll {
    max-height: 320px;
    overflow-y: auto;
  }

  .usage-hint {
    margin-top: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .usage-hint code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    background: var(--bg-primary);
    padding: 2px 6px;
    border-radius: 3px;
  }

  .divider {
    border: none;
    border-top: 1px solid var(--border-default);
    margin: var(--space-6) 0;
  }

  /* Tips */
  .tips {
    padding: var(--space-4);
    background: var(--bg-tertiary);
    border-radius: 8px;
  }

  .tips h5 {
    font-size: var(--text-sm);
    font-weight: var(--font-weight-semibold);
    margin-bottom: var(--space-2);
  }

  .tips ul {
    padding-left: var(--space-5);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: var(--leading-relaxed);
  }

  .tips li {
    margin-bottom: var(--space-1);
  }

  .tips code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    background: var(--bg-primary);
    padding: 1px 4px;
    border-radius: 3px;
  }

  /* Endpoint Form */
  .endpoint-form {
    margin-bottom: var(--space-4);
  }

  .input-row {
    display: flex;
    gap: var(--space-3);
  }

  .input-row .input {
    flex: 1;
  }

  /* Connection Status */
  .connection-status {
    margin-top: var(--space-4);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4);
    border-radius: 8px;
    font-size: var(--text-sm);
  }

  .status-indicator.idle {
    background: var(--bg-tertiary);
    color: var(--text-muted);
  }

  .status-indicator.testing {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .status-indicator.success {
    background: hsla(142, 60%, 45%, 0.1);
    border: 1px solid hsla(142, 60%, 45%, 0.3);
    color: var(--accent-success);
  }

  .status-indicator.failure {
    background: hsla(0, 70%, 55%, 0.1);
    border: 1px solid hsla(0, 70%, 55%, 0.3);
    color: var(--accent-danger);
  }

  .status-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .status-text strong {
    font-weight: var(--font-weight-semibold);
  }

  .status-text span {
    font-size: var(--text-xs);
    opacity: 0.8;
  }
</style>
