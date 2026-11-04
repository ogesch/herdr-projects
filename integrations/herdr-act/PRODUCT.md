# Product

<!-- impeccable:product-schema 1 -->

## Platform

macos

Native macOS Herdr app (`herdr-app`). The ACT view ships as the personal layout `act-workflow` plus native mods (`native/act-overview`, `native/act-spaces`). The mods render through Herdr's GPUI kit (`gpui-kit.d.ts` in the installed mod), not HTML/CSS, so browser-based tooling, CSS rules and live mode do not apply. Impeccable has no native macOS reference and reports this value as unrecognized; treat platform conventions as macOS desktop, not web or iOS. Linux remains a plugin platform for the terminal board but is not the design target of the native view.

Scope: this file covers only the native Herdr ACT overview in `integrations/herdr-act/`. The Shepherdr web app at the repository root is a separate application and is not described here.

## Users

The owner and a small team of developers who run several coding agents in parallel (Codex CLI, Claude Code, Codex Desktop) using the ACT workflow. Design load: about four main agents at once, each with sub-agents, across several projects. Used about equally on large displays and on a 13-inch MacBook.

## Product Purpose

One place to oversee and steer agent work in Herdr. Overview and control carry equal weight:

- **Overview:** see at a glance which agent needs a decision, which has a problem, which is working, and which ACT phase each agent is in.
- **Control:** jump straight into an agent's session, delegate and correct work, close or hide sessions, and open Codex Desktop conversations in Herdr.

Success means the user never has to hunt through terminals to find who needs them, and never mistakes an idle or unknown agent for finished work.

## Positioning

The overview understands the ACT workflow (Interview → Spec → Refine → Create Issues → Implement → Land Feature) and treats agent runtime status, ACT phase, and assignment completion as three separate facts. It combines Herdr's native terminal sessions with Codex Desktop conversations and sub-agent families in one list, scoped by Herdr Space.

## Operating Context

- Lives inside the native Herdr app: a Spaces panel at the top, the agent list in a resizable sidebar (about 400 px, design B "project tree"), and the active tab's terminals in the work area.
- Selecting an agent focuses its existing pane; a child without its own pane opens in Herdr's terminal preview; Codex children sharing a parent terminal open a read-only conversation view.
- ACT commands, answers, and approvals happen in the agent session itself, not through extra buttons in the overview.
- Data arrives from lifecycle hooks, Herdr pane events, the Codex catalog and daemon snapshots; updates are event-driven with cached snapshots.

## Capabilities and Constraints

- Grouping by urgency: needs you, problems, active, done (done and saved conversations start folded).
- Status semantics are fixed: working, needs human, problem, explicitly completed, inactive, unknown. Idle is never completed. Missing or unreliable data shows as "Unbekannt".
- Sub-agents are indented under their parent; "waiting for parent" stays distinct from "needs you".
- Manual corrections (project, title, spec/ticket), hiding, and completion overrides persist locally and survive automatic updates.
- Must stay within what the Herdr plugin and mod API allows; Herdr itself is not modified.
- Must stay close to the upstream package (eliasstravik/herdr-projects, base `plugin-base-v1.2.10`) so upstream changes remain easy to merge. Prefer additive, contained changes over restructuring.
- Must work equally well on a 13-inch MacBook and on large displays.
- UI language is German.

Requirements source: `/Users/oliver/Repos/tools/HERDR_LAYOUT_PLAN.md`. Behaviour reference: `docs/agent-overview.md`.

## Brand Commitments

- Lives under the Herdr / Herdr Projects name; no separate brand.
- UI copy in German (for example "Braucht dich", "Unbekannt", "Schließen …", "Ausblenden", "In Herdr öffnen").
- Owner's stated ambition (binding): a clearly more attractive, modern, polished UI that is easy to use and thought through. Visual direction itself is not decided here.

## Evidence on Hand

- Approved requirements: `/Users/oliver/Repos/tools/HERDR_LAYOUT_PLAN.md`.
- Concept mockups with fictional data: `/Users/oliver/Repos/tools/design-mockups/herdr-2026-10-31/` (A compact list, B project tree, C focus list; B was implemented) and their prompts in `prompts.md`.
- Existing native implementation in `native/act-overview/` and `native/act-spaces/`.
- No user research, usage metrics, or team feedback is recorded; do not invent any.

## Product Principles

1. Never fake completion. Unknown stays unknown, idle is not done, and a finished turn is not a finished assignment.
2. Attention first. A real question to the human always surfaces above everything else, without reinterpreting other agents' status.
3. Overview and control together. Every row that shows a problem or question also leads straight to the place to act on it.
4. Respect the host. Work inside Herdr's native layout and API, and stay close to upstream.
5. Equal on laptop and large display. Density adapts; information does not disappear.

## Accessibility & Inclusion

Status is never conveyed by colour alone: every status has its own symbol (circle, diamond, triangle, check, hollow circle, question mark). Full labels stay available as tooltips when truncated.
