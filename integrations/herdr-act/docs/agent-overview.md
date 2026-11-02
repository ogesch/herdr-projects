# Agent overview

## macOS app

Codex children that share their parent's terminal open a read-only conversation view when selected. It shows the recorded delegated task and the latest 80 visible messages, updates from conversation file events, and returns to the live parent terminal with “Zum Hauptagenten”. Long tasks can be expanded; unavailable or failed reads have explicit states. Late replies never replace a newly selected child. Child labels use their recorded task names instead of “default”; the label index avoids rereading every conversation on each overview refresh.

The overview reacts to native terminal/Space changes and watches local hook records, corrections, the Codex catalog and cached daemon snapshots. Changes are batched; a selected child also follows its recorded parent transcript. Events received during an active read trigger one follow-up. Codex catalog changes during the 30-second cache cooldown schedule one delayed refresh. A 60-second safety check retries failed watchers and recovers missed events; there is no five-second polling. On platforms that cannot watch the configured tree, the sidebar reports the fallback.

Codex daemon reads tolerate temporary socket read timeouts for up to 20 seconds per request, including paginated conversation history. Disconnects and exhausted deadlines still report an error and name the failed request.

Native refreshes read the last successful Codex snapshot immediately. One detached worker updates that private cache, with at least 30 seconds between completed attempts and a shared 90-second daemon-request deadline. A slow daemon cannot consume the app's 30-second command limit or hold up native agent updates. The sidebar indicates an ongoing refresh; failed refreshes retain the previous snapshot and report the source error.

The native app and the original Herdr TUI are separate runtimes. The requested app view is installed as the personal layout `act-workflow`, with `~/.config/herdr-app/mods/act-overview` providing the native panels. It is selected already; use `herdr-app do layout.switch --name act-workflow` to return to it, or select the layout in the app. `main` remains the standard layout. No terminal command inside Herdr TUI and no `HERDR_ENV` override are needed for this view.

The ACT Spaces panel is a personal clone of Herdr's native list. It keeps Space creation, menus, machine groups, and drag ordering, and always shows the detected repository/branch and the first terminal's live working folder independently of the Space's free name.

The agent list defaults to the selected Space. Ownership follows terminal/tab identifiers, not repository names; two Spaces in the same repository stay separate. Children without their own model pane inherit the nearest parent's Space, while children with their own pane follow that pane. Removed ancestors retain their hidden status and surviving descendants are rebased. Clicking a Space resets project filtering and returns to this scoped view.

The single “All Spaces” button lives in the Spaces header and explicitly shows the global agent list, including stored desktop conversations without a Space. Clicking any Space returns to that Space’s agents, even when it was already the active Space. Space navigation and its selected state stay above; the agent list has no second set of Space controls.

The native list uses design B: a compact project tree in a resizable 400-pixel sidebar. Project headers show their main-agent count and most urgent status. Each agent keeps its own status icon, task, program, and ACT phase on one line; children are indented. Only questions and problems add a short second line. Projects and child branches fold independently; newly observed human questions reopen their project and ancestors.

Blue circles mean working, amber diamonds need a human, red triangles mean a problem, and green checks require explicit assignment completion. Hollow gray circles mean inactive; a gray question mark means unknown. Idle is never treated as completed. Completed families and saved Desktop conversations start folded; saved history is paged by whole families and its children start folded.

The work area shows the active tab, including its split panes and ordinary shells. Native agent selection focuses the agent's existing pane; a core child without its own pane opens in Herdr's terminal preview. Closed terminals disappear immediately using the app's live terminal state, independently of the metadata refresh. A click on a stale row refreshes the list without opening the missing terminal. ACT details follow the terminal that actually has focus. Shell and mod panes show no stale agent details. A tab change clears the separate saved-conversation selection and restores the work area. Binding this area to a globally focused agent instead would pull navigation back to the previous tab.

Project filtering, assignment completion overrides, hiding, corrections, and expandable ACT history use the same durable model as the original terminal implementation. Full labels remain available in tooltips. Selecting a Codex App entry shows its details and hides the unrelated terminal. “In Herdr öffnen” resumes the original session ID in a Herdr tab with its existing history, without sending a prompt. A confirmed matching native session is focused instead of opened again. “In Codex Desktop öffnen” keeps the original Desktop link available. Opening a chat does not stop a running Desktop turn; avoid sending messages to the same conversation from both clients at once.

“Schließen …” in the selected agent's details asks for confirmation. Native sessions end their specific terminal and its terminal children; Codex App sessions use `codex archive` with the exact thread UUID, preserving the conversation history. Closing a child never targets its parent. A fresh snapshot rejects confirmation when new work has arrived. Only a successful close records an ended, hidden session; it does not mark the assignment complete. New observed work clears that state. “Ausblenden” is available for every row and only hides it; it leaves the agent running. Hidden rows remain available through “Alle anzeigen”.

The new native adapter reads `herdr-app ls` and `herdr-core` metadata. Hooks validate the real `HC_TERMINAL_ID` and its app labels or parent before recording native CLI events. The Zsh adapter preserves that identity with `codex --no-daemon`. Core child addresses and CLI-internal hook children both join their actual parents. `hc` transport markup is removed from displayed tasks and ACT command detection.

Native source files are under `native/`. `herdr-app check` and actual app screenshots were used; the large Desktop metadata payload is reduced before sending it to the panels. The original terminal checks below describe the earlier TUI implementation, rather than a proof of the native app runtime.

The native app check covers the installed 0.1.9 app, its two existing Codex terminals, and a disposable real core child. Its exact prose question escalated the family while the main agent retained working status. Answering cleared the question; normal responses never completed the assignment. The ten updated Codex observers were reviewed individually, leaving the unrelated pending hook untouched. The disposable child and its test snapshot were removed.

The list keeps the full history in its own memory and shares only native terminals and the selected conversation with the context panel. Native terminals stay in the current project tree, including inactive terminals. Flexible task widths truncate long titles without changing row height; no width-driven render loop is needed. Project indexes are reused until the snapshot or filters change. The panel runs the immutable installed helper, independently of the worktree's build directory.

Design B passed the installed 0.1.10 app's schema check and 26 focused integration tests, including project grouping, independent child status, hidden families, saved history, reopening new questions, and tab/pane navigation. All five existing tabs switched successfully through the app's own commands, including both shell tabs; switching also restored a deliberately hidden work area. The native description check also passed a synthetic 1,000-conversation history. Live screenshots cover 360- and 400-pixel sidebars. The earlier full repository check finished with 994 passing tests and nine previously observed failures outside this view; this navigation fix reran the focused checks. Claude authentication, Desktop live interaction, and the physical 13-inch check remain open.

Open a Herdr-managed terminal in this worktree and run:

    ./target/quick/herdr-projects agents

The pane entry in the plugin also opens this command. CLI sessions retain their existing interactive terminal: Enter attaches to it in a native right split, whose divider Herdr can resize. A Codex App entry opens the original conversation. The overview becomes a full-width list while a terminal is attached. Details and the selected terminal remain to the right at compact widths. Compact entries keep project, runtime, and ACT phase on a separate line instead of clipping them behind long titles.

## Lifecycle and ACT hooks

Install the additive observers with:

    ./target/quick/herdr-projects agents install-hooks

The installer preserves existing hook groups, permissions, environment settings, and trust data. It saves backups before changing either user's settings. Each observer binary has an immutable content-addressed path. Codex requires review of these new hook definitions through its normal /hooks interface; this command never changes trust records or bypasses review. New or resumed Claude sessions load its added hooks.

On this Mac, the installer also appends a guarded source block to .zshrc, with a backup. Only genuine Herdr panes load the Zsh adapter. Interactive Codex starts there use --no-daemon: the app-bundled CLI's shared daemon otherwise loses the pane environment before running hooks and tools. Codex outside Herdr and its daemon/service commands keep their existing behavior. Existing shell aliases/functions are preserved. Open a new pane or start codex --no-daemon explicitly for an already-open shell; existing daemon-backed conversations need a new CLI process.

The common observer handles SessionStart, UserPromptSubmit, tool calls, permission requests, subagent start/stop, turn stop, and session end. Explicit ACT slash/dollar commands and Skill tool invocations select Interview, Spec erstellen, Refine, Create Issues, Implement, or Land Feature. Flutter variants share their phase. Repeated and skipped phases retain the actual invocation order.

Session-start context tells the agent to report automatically entered ACT phases, actual phase ends, known ticket references, questions asked in final prose, and explicit assignment completion. The local reporting command is agents report --source codex|claude --session ID, with --phase COMMAND, --phase-state end, --ticket REF, --question TEXT, or --state done|open. A phase end or Stop hook alone never marks the assignment done. Außerhalb ACT requires an explicit outside-act phase; absent signals remain Unbekannt.

Lifecycle snapshots are private files under the Herdr Projects data folder's agent-overview/sessions directory. Writers use per-session locks and atomic replacement; user corrections use a separate file. Full hook payloads and tool output are not saved. CLI hooks outside Herdr are ignored; Desktop identity is checked against the session's own originator metadata. ACT's per-repository .act/config.yaml selects GitHub references or local.path files; local references cannot escape the configured directory.

## Runtime sources and subagents

The reader connects by WebSocket to the existing local Codex daemon's Unix socket. It only lists and reads metadata; it does not start, resume, archive, or approve a conversation. Desktop conversations hosted in another app-server can appear as notLoaded there. Their runtime, questions, and approvals come from the observed lifecycle hooks; without an observed signal the view keeps Unknown.

CLI roots are limited to sessions found in Herdr. Hook parent identifiers attach Codex and Claude subagents beneath their existing main agent, with their own activity and question text. A child decision escalates the family's attention group while preserving the main agent's own runtime. SubagentStop means waiting for the parent, rather than a human question or completed assignment. Children do not inflate the main-agent count.

Use agents check for source counts without printing thread names, messages, or identifiers.

## Controls and corrections

Up/Down selects a visible row. Enter opens it. p cycles project filters. Keys 1–5 fold attention groups; Space folds the selected agent's children. w expands the full ACT workflow history; PageUp/PageDown scroll the details. e edits title, project, ticket, and phase; m cycles manual completed, manual not completed, and automatic assignment-completion tracking. h hides or restores completed work, and a includes hidden families. New observed work brings a hidden assignment back. q closes the overview.

Corrections survive source refreshes and restarts. Automatic and manual phase records remain distinguishable. Completed, observed CLI assignments remain available after their agent exits; ended rows cannot attach to a replacement process. Hidden assignments are retained locally; their original conversation is not archived or deleted.

## Verified locally on 2026-10-30

The focused project check passed 22 integration tests. Three core tests and the board rendering test also passed. Synthetic rendering covers four parent/child families at 80×32, 120×36, and 180×50. The full repository check had 987 passing tests and nine failures also observed before these final corrections; those unrelated failures remain open.

An isolated real Herdr session supplied four main agents across two repositories: three Codex CLIs and one unauthenticated Claude CLI. Three actual Codex children were linked to their parents and ended as waiting for the parent. A real Codex prose question was reported, persisted through Stop, and cleared after answering through the attached original terminal. That bounded test assignment then reported explicit completion. The active compact layout measured 114×39, with two interactive 57-column panes; project, runtime, ACT state, question, and the original conversation were visible. This is a terminal-size check, not a physical 13-inch display check.

Claude emitted startup and its actual authentication failure. The user deferred its authenticated question/subagent test. The existing Codex daemon supplied 722 Desktop metadata threads as notLoaded. Desktop hook runtime and opening the matching original app conversation remain unverified in the native UI; no Desktop session was resumed to manufacture live state. The source changes are local and the installed release's plugin registration has not been replaced; use the direct command above.
