# Native Herdr ACT integration

The native macOS Herdr ACT layout lives in
[`integrations/herdr-act/`](../integrations/herdr-act/). This is the complete Rust
workspace from `plugin-base-v1.2.10`, including the ACT backend, lifecycle observers,
terminal overview, native layout, Space panel, and regression tests.

The repository root remains the standalone Shepherdr 2.1 TypeScript application.
The Rust integration is a separate application for the native Herdr host. It is
not a Shepherdr 2.1 plugin and does not share that application's state or plugin API.

The native layout keeps Spaces above the agents. Selecting a Space filters agents
by their actual terminal and tab ownership. The single **All Spaces** button in
the Spaces header shows the global list. Subagent relationships, ACT history,
assignment completion, saved conversations, and the cached Codex source retain
their existing behavior.

## Build and use

From the repository root:

```sh
cd integrations/herdr-act
cargo build --locked --profile quick
./target/quick/herdr-projects agents
```

The native modules and layout are in `native/`. Their configured helper path refers
to an installed immutable binary. Moving their source into this package does not
reinstall hooks, replace that binary, or switch the running application's layout.
Installation, lifecycle reporting, native navigation, and verification limits are
documented in the package's [agent overview guide](../integrations/herdr-act/docs/agent-overview.md).
Its historical README and development instructions describe this Rust package.

## Verification

The root Check workflow runs the Rust package's original `npm run check -- --all`
alongside Shepherdr's TypeScript, browser, and build checks. This checks the package
from its nested path, including the native JavaScript regression fixtures.
Follow the root [agent instructions](../AGENTS.md) for local verification; full
suites run in GitHub. Neither the merge nor this workflow deploys either application
or migrates private state.
