# Figurine Studio

Pixel art character pipeline tool for game development. Import MagicaVoxel renders, apply AI-powered texture generation, remove backgrounds, compose spritesheets, and export assets for Bevy Engine.

## Features

- **Project Management** -- Organize characters by project with shared settings (tile size, animation definitions, style prompts)
- **Image Import** -- Drag-and-drop import of MagicaVoxel rendered images with automatic sprite slot assignment
- **AI Texture Generation** -- ComfyUI integration for ControlNet-based pixel art texture generation
- **Background Removal** -- ONNX Runtime (U2-Net) powered background removal, batch or per-sprite
- **Spritesheet Generation** -- Automatic spritesheet composition from direction/animation grid
- **Bevy Export** -- One-click export of spritesheet PNG and JSON metadata for Bevy Engine
- **Animation Preview** -- Real-time sprite animation preview with zoom, speed control, and grid overlay

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop Runtime | Tauri v2 |
| Frontend | SvelteKit + Svelte 5 (runes) |
| Backend | Rust |
| Database | SQLite (via rusqlite) |
| AI Inference | ONNX Runtime (background removal) |
| AI Generation | ComfyUI (texture generation) |
| UI Icons | lucide-svelte |

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- [pnpm](https://pnpm.io/)
- ComfyUI (optional, for AI texture generation)

## Quick Start

```bash
# Install frontend dependencies
pnpm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Project Structure

```
rs-figure-studio/
  src/                          # SvelteKit frontend
    app.css                     # Global styles, theme variables
    routes/
      +layout.svelte            # App shell (sidebar, toast, setup wizard)
      +page.svelte              # Home (project list)
      project/
        new/+page.svelte        # Create project
        [id]/+page.svelte       # Project detail & character list
      character/
        new/+page.svelte        # Create character
        [id]/
          +page.svelte          # Character detail
          +layout.svelte        # Character layout (tabs)
          import/+page.svelte   # Image import
          process/+page.svelte  # AI processing & BG removal
          preview/+page.svelte  # Preview & export
      settings/
        +page.svelte            # Settings hub (theme, export path)
        comfyui/+page.svelte    # ComfyUI connection settings
        models/+page.svelte     # ONNX model management
    lib/
      api/tauri.ts              # Tauri invoke wrappers
      types/index.ts            # TypeScript type definitions
      stores/                   # Svelte 5 runes-based state
        project.svelte.ts
        character.svelte.ts
        sprite.svelte.ts
        toast.svelte.ts
        settings.svelte.ts
        comfyui.svelte.ts
        onnx.svelte.ts
        setup.svelte.ts
      components/               # Reusable UI components
        Sidebar.svelte
        Breadcrumb.svelte
        SpritePreview.svelte
        Toast.svelte
        ConfirmDialog.svelte
        LoadingSpinner.svelte
        EmptyState.svelte
        ErrorBoundary.svelte
        ImageDropZone.svelte
        ProgressBar.svelte
        WorkflowManager.svelte
        ModelDownloader.svelte
        SetupWizard.svelte
  src-tauri/                    # Rust backend
    src/
      main.rs                   # Entry point
      lib.rs                    # App builder & plugin registration
      state.rs                  # App state (DB, ONNX session)
      error.rs                  # Error types
      commands/                 # Tauri command handlers
      models/                   # Data models
      services/                 # Business logic
      db/                       # Database migrations & queries
```

## Usage

1. **Create Project** -- Set project name, tile size (e.g., 64x64), directions (down/left/right/up), and animations (idle/walk/attack)
2. **Add Character** -- Create a character within the project (player/enemy/NPC)
3. **Import Images** -- Drag MagicaVoxel renders into the import page; files are auto-assigned to direction/animation/frame slots
4. **Process** -- (Optional) Run AI texture generation via ComfyUI, then batch background removal via ONNX
5. **Preview** -- View animated sprite preview, adjust zoom and playback speed
6. **Export** -- Generate spritesheet and export for Bevy with a single click

## License

Private project.
