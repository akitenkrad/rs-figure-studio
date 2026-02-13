<script lang="ts">
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import ModelDownloader from '$lib/components/ModelDownloader.svelte';

  const breadcrumbItems = [
    { label: 'ホーム', href: '/' },
    { label: '設定', href: '/settings' },
    { label: 'モデル管理' },
  ];
</script>

<Breadcrumb items={breadcrumbItems} />

<div class="models-page">
  <h1>モデル管理</h1>
  <p class="page-description">
    Figurine Studio で使用するAIモデルの管理を行います．
    背景除去にはONNXモデル（U2-Net）が必要です．
  </p>

  <section class="model-section">
    <h2>背景除去モデル</h2>
    <ModelDownloader />
  </section>

  <section class="info-section">
    <h3>手動配置について</h3>
    <div class="info-card">
      <p>
        オフライン環境やファイアウォール制限がある場合は，手動でモデルファイルを配置できます．
      </p>
      <ol class="manual-steps">
        <li>
          別のマシンで以下のURLからモデルをダウンロード:<br />
          <code class="url-code">
            https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx
          </code>
        </li>
        <li>
          ダウンロードした <code>u2net.onnx</code> を以下のディレクトリに配置:<br />
          <code class="path-code">
            ~/Library/Application Support/com.figurine-studio.app/models/
          </code>
        </li>
        <li>アプリを再起動</li>
      </ol>
    </div>
  </section>
</div>

<style>
  .models-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  .models-page h1 {
    margin-bottom: var(--space-1);
  }

  .page-description {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-relaxed);
  }

  .model-section h2 {
    margin-bottom: var(--space-4);
  }

  .info-section h3 {
    margin-bottom: var(--space-3);
  }

  .info-card {
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .info-card p {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-3);
  }

  .manual-steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-left: var(--space-5);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: var(--leading-relaxed);
  }

  .url-code,
  .path-code {
    display: inline-block;
    margin-top: var(--space-1);
    padding: var(--space-1) var(--space-2);
    background: var(--bg-tertiary);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-primary);
    word-break: break-all;
  }
</style>
