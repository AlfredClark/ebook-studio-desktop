English | [简体中文](docs/README_zh-CN.md)

# tauri-desktop-template

Desktop application development template based on Tauri v2 + SvelteKit 2 + Svelte 5.

Production-ready starting point: type-safe Tauri commands, theming, dual-track i18n, crash
safety nets, and a desktop shell (tray, autostart, window state, single instance, deep link,
updater) out of the box.

> The capability demo page (`/demo`) and the SQLite notes sample (`/notes`) used to ship with
> this template; both were removed to keep the main branch a clean base. Their full wiring
> (sandbox fs, native dialogs, notifications, global shortcuts, network requests, file drop)
> lives in the [`v0.3.0`](https://github.com/AlfredClark/tauri-desktop-template/tree/v0.3.0) tag.

## Screenshots

| Home (`/`)                    | Settings (`/settings`)                | About (`/about`)                |
| ----------------------------- | ------------------------------------- | ------------------------------- |
| ![Home](docs/images/home.png) | ![Settings](docs/images/settings.png) | ![About](docs/images/about.png) |

> Screenshots live in `docs/images/` and are named after their route (`home` / `settings` / `about`).

## Features

1. **Type-safe command chain (tauri-specta)** — Rust `#[tauri::command]` +
   `#[specta::specta]` exports types into `src/libs/commands/bindings.ts` (generated, never
   hand-edited). Frontend calls go through the chained API in `src/libs/commands` only —
   no bare `invoke` in components:
   - Read value with fallback: `commands.getConfig().value(fallback)`
   - Branch on status: `commands.getConfig().result()`
   - Transaction style: `commands.updateConfig({ locale }).success(onOk).failed(onFail)`
   - Unregistered `.failed()` failures are reported via `plugin-log` instead of being
     silently dropped.
2. **Dark/light mode + color themes** — `mode-watcher` follows the system (`light` / `dark` /
   `system`) with a Tailwind v4 `.dark` variant; ten palettes (vivid `ocean` / `teal` /
   `emerald` / `amber` / `tangerine` / `rose` / `amethyst` + dark-aesthetic `nord` / `dracula` /
   `tokyo-night`, `neutral` falling back to defaults) switch via the `data-color-theme`
   attribute (`themes.css`, deliberately avoiding `data-theme` which mode-watcher overwrites
   on mount). Pure frontend preference, persisted in `localStorage`, applied before first paint.
3. **Dual-track i18n (Paraglide + rust-i18n)** — frontend `paraglide-js` (`en` / `zh-CN`),
   backend `rust-i18n` (`src-tauri/locales/*.yml`, fallback `en`). The single source of truth
   is the backend `config.json` `locale` key: backend reads persisted value (or probes the OS
   locale via `tauri-plugin-os` on first run), frontend hydrates it in the background on
   `+layout.svelte` mount (deliberately not in `+layout.ts` `load()` — Tauri `invoke` can't
   plug into the `fetch` SvelteKit hands to `load`) and aligns with `alignLocale` from
   `libs/i18n/locale.svelte.ts` (writes the Paraglide localStorage cache and re-renders the
   already-rendered copy in place, no full reload).
4. **Crash safety nets, frontend + backend** — frontend `ErrorBoundary` (reports via
   `@tauri-apps/plugin-log`, stack trace in dev only, 2s throttle); backend panic hook in
   `cores/system.rs` (normal log first, temp-dir `my_app_crash.log` fallback with 512KB rotation; `%TEMP%` on Windows, `$TMPDIR` on Unix).
5. **Settings page (`/settings`)** — General group (language, autostart, remember window,
   auto-check updates, tray, close behavior with `prompt` / `exit` / `minimize_to_tray`) backed
   by `config.json` via `updateConfig`; Appearance group (theme, color theme, `tabs` / `sidebar`
   layout, system-font picker, weight, size) is frontend-only; Shortcut group (show-app:
   badge-plus display + key recording, sticky error on register failure); one-click reset to defaults.
6. **About page (`/about`)** — app info, project links (`opener`, `http(s)` only), platform
   info, diagnostics (open log/config dirs, copy system info), and an updater panel
   (check → download with progress → restart; desktop only, 120s check timeout).
7. **Desktop shell** — frameless window with custom title bar (pin / minimize / maximize /
   close), system tray with localized menu (show/hide, quit), close-behavior interception,
   autostart, window-state restore (window starts `visible: false`, shown after restore),
   single-instance focus + deep-link arg forwarding (`tdt://` scheme), show-app global shortcut
   (`shortcut_show` config-driven, sticky failure toast), updater wired to GitHub
   Releases `latest.json`.

## Tech stack

> Version source of truth: `package.json` (frontend) and `src-tauri/Cargo.toml` (backend).
> Always code against the versions declared there.

| Layer           | Choice                                                                                                                                                                    | Notes                                                                                                                                                             |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Frontend        | SvelteKit 2 + Svelte 5                                                                                                                                                    | SPA mode (`adapter-static` + `fallback: index.html`, `ssr = false`), Vite port 1420                                                                               |
| UI              | shadcn-svelte (nova / neutral) + Tailwind CSS v4, Lucide icons, Geist font                                                                                                | `cn()` class merge, `.dark` variant + `[data-color-theme]` palettes                                                                                               |
| Bridge          | `@tauri-apps/api`, `plugin-log` / `opener`, `system-fonts`                                                                                                                | Logs via `plugin-log`, font list via `system-fonts-api`                                                                                                           |
| Contract        | tauri-specta + `specta` / `specta-typescript`                                                                                                                             | Generated `src/libs/commands/bindings.ts`, do not hand-edit                                                                                                       |
| i18n            | Paraglide (frontend) + rust-i18n (backend)                                                                                                                                | `src/libs/i18n/messages/*.json` + `src-tauri/locales/*.yml`                                                                                                       |
| Backend         | Tauri v2 + Rust (edition 2024, pinned toolchain in `rust-toolchain.toml`)                                                                                                 | `commands/` thin wrappers (by call domain), `features/` per-business logic (runtime allowed), `cores/` shared abilities                                           |
| Backend plugins | `opener` / `store` / `log` / `os` / `updater` / `autostart` / `window-state` / `system-fonts` / `single-instance` / `deep-link` / `clipboard-manager` / `global-shortcut` | `updater` / `autostart` / `window-state` / `single-instance` / `deep-link` / `global-shortcut` desktop only (`global-shortcut` Rust-side only, no frontend calls) |
| Quality         | TypeScript strict + `svelte-check`, ESLint + Prettier, vitest, pnpm                                                                                                       | Node `>= 24`, `pnpm-lock.yaml` + `Cargo.lock` committed                                                                                                           |
| Release         | GitHub Actions (`ci.yml` / `release.yml`) + Tauri bundler                                                                                                                 | Version triplet kept in sync (see below)                                                                                                                          |

## Quickstart

Prerequisites: Node `>= 24`, `pnpm@12.15.0`, Rust toolchain from `rust-toolchain.toml`.

```bash
pnpm install --frozen-lockfile   # install deps (never npm / yarn)

pnpm tauri:dev                   # full desktop dev: Vite (port 1420) + Tauri window
pnpm dev                         # frontend only (Tauri commands degrade gracefully)

pnpm format                      # fix both sides: Prettier + ESLint + cargo fmt
pnpm validate                    # test + lint + check (run after every change)

pnpm build                       # frontend only (adapter-static → build/)
pnpm tauri:build                 # full packaging; tauri:build:local compiles without bundling
```

Common scripts:

```bash
pnpm lint            # frontend + backend checks
pnpm check           # svelte-kit sync + svelte-check (strict TS)
pnpm test            # vitest + cargo test (single run)
pnpm test:cargo      # backend tests + regenerate bindings.ts
pnpm i18n:compile    # required after editing messages/
pnpm tauri:icon      # regenerate icons from src-tauri/app-icon.svg
pnpm release         # bumpp linked versions: package.json + tauri.conf.json + Cargo.toml
```

## Pages

| Route       | Contents                                                                                        |
| ----------- | ----------------------------------------------------------------------------------------------- |
| `/`         | Hero: stack icons + app name + intro. Layout-skeleton starting point, build your business here. |
| `/settings` | General (backend-persisted) + Appearance (frontend-only) + Shortcut (global register) + reset.  |
| `/about`    | App / project / platform / diagnostics groups + updater panel.                                  |

Navigation tabs are registered once in `src/libs/navigation/nav-tabs.ts` and shared by the
`tabs` and `sidebar` layouts (`src/components/layout/`); add a page by adding one entry there.
Appearance (layout registry + font preference) lives in `src/libs/hooks/appearance.svelte.ts`.

## Configuration

Backend `config.json` (via `tauri-plugin-store`, `cores/config.rs`) holds the durable settings:

- `locale` (`"en" | "zh-CN"`, specta union — no bare `string`), `auto_start`,
  `remember_window`, `auto_check_update`, `tray_enabled`,
  `close_behavior` (`prompt` / `exit` / `minimize_to_tray`), `shortcut_show` (show-app global shortcut, empty means unset), `schema_version`.
- Write paths go through `update` / `save_locale` / `migrate` only; `update` runs an idempotent
  `migrate` inside. Adding/changing a field means updating `CURRENT_SCHEMA_VERSION` +
  `MIGRATIONS` when existing-field semantics change.
- Frontend-only appearance (theme, color theme, layout, font family/weight/size) stays in
  `localStorage` and never crosses the command bridge — an intentional exception.

## Workflows

**Add/change a command:** with a matching business, `features/<name>.rs` implements it (declare in `features/mod.rs`); pure shared needs call `cores/` directly →
`commands/<name>.rs` thin wrapper (`CommandResult` + both attributes + `collect_commands!`) →
`cargo test` regenerates `bindings.ts` → frontend chained-API call → joint debug. Never
hand-edit `bindings.ts`.

**Edit copy:** frontend `src/libs/i18n/messages/*.json` → `pnpm i18n:compile` → use `m.<key>()`
/ `setLocale()`; backend `src-tauri/locales/*.yml` → `rust_i18n::t!(...)`.

**Bootstrap a new project:** rebranding, deep-link scheme and signing-key rotation, icons,
doc cleanup and the acceptance matrix are described end to end in `docs/INITIAL.md`. Scaffolding
removal is already done in this branch — if you want reference implementations for the desktop
plugins (sandbox fs, dialogs, notifications, global shortcuts, network, file drop), check the
[`v0.3.0`](https://github.com/AlfredClark/tauri-desktop-template/tree/v0.3.0) tag.

## Project structure (condensed)

```text
src/routes/(main)/         # /, /settings, /about pages; page components in `_components/` per route
src/components/layout/    # tabs / sidebar / dashboard shells + title-bar / nav parts
src/components/common/    # shared hand-written components (card-section / card-row / confirm-dialog)
src/libs/commands/        # bindings.ts (generated) + chained-API wrapper
src/libs/i18n/            # messages/ copy + project.inlang/ config
src/libs/navigation/      # nav-tabs.ts single-source nav registry
src-tauri/src/commands/   # thin wrappers + collect_commands!
src-tauri/src/features/   # pure business logic (no Tauri runtime)
src-tauri/src/cores/      # config / locale / system / tray / updater / ...
src-tauri/src/plugins/    # per-plugin init
src-tauri/locales/         # tray.yml (`tray.*`), en + zh-CN per file
```

See `AGENTS.md` for the always-on rules and security red lines; `docs/ARCHITECTURE.md` and
`docs/WORKFLOW.md` for structure, workflows, and the design invariants behind them.

## Validation & contributing

- After any change: `pnpm format`, then `pnpm validate` — both green before claiming “done”.
  Backend command changes additionally need `pnpm test:cargo` + a `bindings.ts` diff check;
  copy changes need `pnpm i18n:compile`.
- Commits follow Conventional Commits in lowercase English
  (e.g. `feat(i18n): persist locale in backend`), one concern per commit; never auto-commit.
- Release: PR → green CI → squash merge → `pnpm release` (linked triplet:
  `package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml`) → tag `vX.Y.Z` →
  `release.yml` builds → `pnpm changelog`.

## License

GPL-3.0-only — see [LICENSE](LICENSE).
