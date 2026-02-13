<script lang="ts">
  import { Check, X, Loader2, Terminal, Download, Wifi } from 'lucide-svelte';

  type GuideTab = 'install' | 'launch' | 'test';

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

  // OS detection via navigator
  const currentOS = $derived(
    (() => {
      if (typeof navigator === 'undefined') return 'linux';
      const ua = navigator.userAgent.toLowerCase();
      if (ua.includes('mac') || ua.includes('darwin')) return 'macos';
      if (ua.includes('win')) return 'windows';
      return 'linux';
    })(),
  );

  // OS-specific install commands
  const installCommands: Record<string, string> = {
    macos: `# Homebrew経由でPythonをインストール
brew install python@3.11

# ComfyUIをクローン
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI

# 仮想環境を作成して有効化
python3 -m venv venv
source venv/bin/activate

# 依存関係をインストール
pip install -r requirements.txt`,
    windows: `# ComfyUIをクローン
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI

# 仮想環境を作成して有効化
python -m venv venv
venv\\Scripts\\activate

# 依存関係をインストール
pip install -r requirements.txt

# またはComfyUI Desktop版をダウンロード
# https://github.com/comfyanonymous/ComfyUI/releases`,
    linux: `# ComfyUIをクローン
git clone https://github.com/comfyanonymous/ComfyUI.git
cd ComfyUI

# 仮想環境を作成して有効化
python3 -m venv venv
source venv/bin/activate

# 依存関係をインストール
pip install -r requirements.txt`,
  };

  // OS-specific launch commands
  const launchCommands: Record<string, string> = {
    macos: `cd ComfyUI
source venv/bin/activate
python main.py --listen 127.0.0.1 --port 8188`,
    windows: `cd ComfyUI
venv\\Scripts\\activate
python main.py --listen 127.0.0.1 --port 8188`,
    linux: `cd ComfyUI
source venv/bin/activate
python3 main.py --listen 127.0.0.1 --port 8188`,
  };

  const osLabel: Record<string, string> = {
    macos: 'macOS',
    windows: 'Windows',
    linux: 'Linux',
  };

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
        <p class="os-info">
          お使いのOS: <strong>{osLabel[currentOS]}</strong>
        </p>

        <div class="code-block">
          <pre><code>{installCommands[currentOS]}</code></pre>
        </div>

        <div class="note">
          <p>
            ControlNet と IP-Adapter のカスタムノードもインストールが必要です．
            ComfyUI Manager の利用を推奨します．
          </p>
          <ol class="install-steps">
            <li>ComfyUI Manager をインストール:
              <code>cd custom_nodes && git clone https://github.com/ltdrdata/ComfyUI-Manager.git</code>
            </li>
            <li>ComfyUI を再起動し，Manager から ControlNet ノードを検索してインストール</li>
            <li>必要なモデル（ControlNet，チェックポイント）を <code>models/</code> に配置</li>
          </ol>
        </div>
      </div>
    {:else if activeTab === 'launch'}
      <div class="tab-panel">
        <h4>ComfyUI 起動コマンド</h4>

        <div class="code-block">
          <pre><code>{launchCommands[currentOS]}</code></pre>
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

  .os-info {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-4);
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

  .install-steps {
    margin-top: var(--space-2);
    padding-left: var(--space-5);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: var(--leading-relaxed);
  }

  .install-steps li {
    margin-bottom: var(--space-1);
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
