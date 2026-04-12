# Repository Guidelines

## Project Structure & Module Organization
- `crates/`: Rust workspace crates — `server` (API + bins), `db` (SQLx models/migrations), `executors`, `services`, `utils`, `deployment`, `local-deployment`, `remote`, `git`, `review`, `api-types`.
- `frontend/`: React + TypeScript app (Vite, Tailwind). Source in `frontend/src`:
  - `components/` — UI components organized by scope: `legacy-design/`, `ui-new/` (new design system with views/primitives/containers/hooks separation), `common/`, `dialogs/`, `agents/`, `panels/`.
  - `hooks/` — Data fetching and state management hooks (70+ hooks for queries, mutations, and business logic).
  - `contexts/` — React contexts for global state (auth, project, workspace, etc.).
  - `stores/` — Zustand stores for UI preferences and expandable states.
  - `pages/` — Route-level page components.
  - `lib/` — Utilities (API client, auth, colors, modals, types, routes).
  - `types/`, `constants/`, `config/` — Type definitions, constants, configuration.
  - `i18n/` — i18n configuration.
- `remote-frontend/`: Separate frontend for remote deployment.
- `shared/`: Generated TypeScript types (`shared/types.ts`, `shared/remote-types.ts`, `shared/schemas/`). Do not edit directly.
- `assets/`, `dev_assets_seed/`, `dev_assets/`: Packaged and local dev assets.
- `npx-cli/`: Files published to the npm CLI package.
- `scripts/`: Dev helpers (ports, DB preparation).
- `docs/`: Documentation files.

## Managing Shared Types Between Rust and TypeScript

**Type Generation** — `ts-rs` (git fork: `xazukx/ts-rs` branch `use-ts-enum`) derives TypeScript types from Rust structs/enums. Annotate Rust types with `#[derive(TS)]` and related macros. Generated types are written to `shared/types.ts` and `shared/remote-types.ts`.

**Workflow**:
1. Edit Rust type definitions (in `crates/api-types/src/*.rs`, `crates/server/src/**`, etc.).
2. Run `pnpm run generate-types` to regenerate `shared/types.ts` or `pnpm run remote:generate-types` for remote types.
3. Do NOT manually edit `shared/types.ts` — instead edit the source Rust types or `crates/server/src/bin/generate_types.rs` if custom logic is needed.

**Key points**:
- Types are re-exported from `api-types` crate and used across services.
- Frontend imports from `shared/types.ts` for type safety.
- Remote deployment uses separate `shared/remote-types.ts`.

## Build, Test, and Development Commands

### Core Commands
- Install: `pnpm i`
- Run dev (frontend + backend with auto-assigned ports): `pnpm run dev`
- Run dev (release build): `pnpm run dev:release` (uses production DB; changes persist to real data)
- Backend (watch mode): `pnpm run backend:dev:watch` (rebuilds on source changes)
- Backend (release build watch): `pnpm run backend:dev:watch:release`
- Frontend dev: `pnpm run frontend:dev` (serves on `FRONTEND_PORT`, proxies `/api` to `BACKEND_PORT`)
- Type checks: `pnpm run check` (frontend TypeScript) and `pnpm run backend:check` (Rust cargo check)
- Linting: `pnpm run lint` (frontend ESLint + backend clippy), `pnpm run lint:fix` (auto-fix)
- Formatting: `pnpm run format` (cargo fmt + Prettier)

### Type Generation & Database
- Generate TS types from Rust: `pnpm run generate-types` (or `generate-types:check` in CI)
- Remote types: `pnpm run remote:generate-types` or `remote:generate-types:check`
- Prepare SQLx (offline): `pnpm run prepare-db` (or `prepare-db:check` in CI)
- Prepare SQLx (remote, PostgreSQL): `pnpm run remote:prepare-db` (or `remote:prepare-db:check`)

### Rust Testing & Building
- Run tests: `cargo test --workspace`
- Build NPX package locally: `pnpm run build:npx` then `pnpm pack` in `npx-cli/`

### Remote Development
- Dev with Docker (remote deployment): `pnpm run remote:dev` (spins up docker compose, cleans up on exit)
- Clean up remote containers: `pnpm run remote:dev:clean`

### Frontend Build Pipeline
- Vite with React plugin (includes babel-plugin-react-compiler for automatic memoization).
- Custom Vite plugin loads executor schemas from `shared/schemas/` as virtual module.
- Aliases: `@/` → `frontend/src/`, `shared/` → `shared/`.
- Source maps enabled in production builds.

## Coding Style & Naming Conventions

### Rust
- `rustfmt` enforced (`rustfmt.toml`): edition 2024, `group_imports = "StdExternalCrate"`, `imports_granularity = "Crate"`.
- snake_case modules, PascalCase types.
- Keep functions small, add `Debug`/`Serialize`/`Deserialize` where useful.

### TypeScript/React
- ESLint + Prettier: 2 spaces, single quotes, max 80 cols.
- File naming: PascalCase for `.tsx` components, camelCase for hooks/utils/lib/config/constants, kebab-case for shadcn UI components (`src/components/ui/**`).
- Components follow strict architecture:
  - **Presentational** (`src/components/ui-new/views/**`, `src/components/ui-new/primitives/**`): Stateless, props-only, no hooks (useState, useContext, useQuery, API calls), no JSX in logic files.
  - **Container** (`src/components/ui-new/containers/**`): Manage state, data fetching, business logic; required props (no optionals).
  - **Logic hooks** (`src/components/ui-new/hooks/**`): Pure functions, no JSX returned, only data and callbacks.
  - **UI-new components** must use `@phosphor-icons/react` (not lucide-react), icon sizes via design system (`size-icon-xs`, `size-icon-sm`, `size-icon-base`, `size-icon-lg`, `size-icon-xl`).
  - Barrel exports banned in `ui-new` (no re-exports via `index.ts`).
- Dialogs: Use typesafe pattern via `lib/modals.ts`; call `DialogName.show(props)` and `DialogName.hide()` (not `NiceModal.show()`).

## Testing Guidelines

### Rust
- Prefer unit tests alongside code using `#[cfg(test)]` blocks.
- Run all tests with `cargo test --workspace`.
- Add tests for new logic and edge cases.
- Common testing tools: `anyhow::Result`, custom error types via `thiserror`.

### Frontend
- TypeScript checks: `pnpm run check` (via `tsc --noEmit`).
- Linting: `pnpm run lint` (ESLint with strict rules).
- Prettier formatting: Automatically enforced by ESLint.
- Optional lightweight tests: Vitest (not commonly used; prefer integration testing via dev server).
- Test files exception: `**/*.test.{ts,tsx}`, `**/*.stories.{ts,tsx}` exempt from i18n and literal string rules.

### ESLint Enforcement
- Unused imports/variables: `error` (catches unused-imports, noUnusedLocals, noUnusedParameters).
- Switch exhaustiveness: enforced (catch missing cases).
- Modal pattern: restricted to `DialogName.show(props)`, not `NiceModal.show()`.
- Component constraints: Presentational components cannot use state, hooks, or data fetching.
- Icon usage: ui-new components must use Phosphor icons and design system sizes.

## Frontend Data Flow & API Integration

### Hooks & Queries
- **React Query** (TanStack) for data fetching (70+ custom hooks in `frontend/src/hooks/`).
- **Zustand** for UI state (stores in `frontend/src/stores/`).
- **React Router** for navigation.
- **Context API** for global state (auth, project context, terminal, review, etc.).

### API Client
- `lib/api.ts` — REST client using `fetch` for `/api` routes (proxied via Vite).
- WebSocket support via `react-use-websocket` for streaming data (logs, diffs).
- Shared types imported from `shared/types.ts`.

### Component Data Patterns
- **Containers** fetch data (via hooks), compute derived state, pass to presentational components.
- **Presentational** components receive all data as props (no API calls, no hooks).
- Dialogs implement the typesafe modal pattern via `lib/modals.ts`.

### i18n
- Internationalization via `i18next` + `react-i18next`.
- Literal strings in JSX checked by ESLint when `LINT_I18N=true` (`pnpm run lint:i18n`).
- Excluded from checks: `data-testid`, `to`, `href`, `id`, `key`, `type`, `role`, `className`, `style`.

## Security & Config Tips
- Use `.env` for local overrides; never commit secrets. Key envs: `FRONTEND_PORT`, `BACKEND_PORT`, `HOST`.
- Dev ports and assets are managed by `scripts/setup-dev-environment.js`.
- Error tracking: Sentry integration in both frontend and backend.
- Workspace limits enforced to prevent resource exhaustion.

