# Aether OS — Repository Rules for Coding Agents

## Mission
Improve this existing Aether OS codebase safely. Priorities, in order:
1. Prevent security flaws and unsafe behavior.
2. Fix correctness bugs and broken existing functionality.
3. Improve UI usability, consistency, responsiveness, and accessibility where UI code exists.
4. Reduce RAM, storage, startup, and runtime overhead using measurements or clear code evidence.
5. Improve maintainability and tests.

Do not add AI integrations or AI-powered features. Do not replace the project architecture just to make the code look cleaner.

## Start by understanding the repository
- Read the README, directory tree, build files, tests, and relevant source before editing.
- Identify the actual OS architecture, language(s), build/test commands, and current working state from the repository. Do not assume a file, subsystem, command, or feature exists until verified.
- Check `git status --short` before making changes. Preserve user changes; never reset, clean, overwrite, or discard unrelated work.
- If this is a Rust project, follow its existing edition, crate boundaries, error-handling style, and formatting. Run `cargo fmt --check`, relevant `cargo check`, and focused tests when supported by the repository.

## Work in small, reversible steps
- Make one logical change at a time and verify it before starting another.
- Prefer a focused patch over a broad rewrite. Avoid unrelated formatting or renames.
- Before modifying a subsystem, explain briefly what evidence points to the problem and what change is planned.
- Keep a running record in `PROGRESS.md`: objective, files changed, commands run, results, remaining risks, and next exact step. Update it after each completed unit of work.
- Make a checkpoint summary every 2 completed units of work.
- If Git is available and the initial working tree is clean, create a descriptive branch only after telling the user what you intend to do. Do not commit unless explicitly asked.

## Anti-loop / anti-garbage-output rules
- Never repeat an identical failing command more than twice. After the second failure, inspect the output, diagnose, and either try a meaningfully different fix or stop that task.
- If output becomes repetitive (same character, line, number, or code block), stop generation/editing immediately, inspect the last diff and logs, then resume with a short, concrete subtask. Do not append more generated text to a suspected loop.
- Do not repeatedly rewrite the same file without checking `git diff`.
- Do not run blind retries, unbounded loops, recursive destructive commands, or unbounded downloads/builds.
- Avoid huge multi-file edits in one response. Keep changes small enough to review.
- If the same blocker remains after two distinct approaches, record it in `PROGRESS.md` and move to a different independent task or stop.
- Never claim a test, build, security check, or performance improvement passed unless the command output supports that claim.

## Security and destructive actions
- Treat input parsing, memory access, filesystem operations, permissions, process boundaries, networking, authentication, and update/install paths as security-sensitive when present.
- Validate untrusted input at the boundary; avoid unsafe operations where practical; document unavoidable unsafe code and its invariants.
- Never add hard-coded credentials, private keys, tokens, or personal data.
- Never weaken security checks merely to make tests pass.
- Do not delete user files, wipe disks, format partitions, change boot records, run privileged system modifications, alter host firewall settings, or execute installers without explicit approval.
- Do not execute a binary produced by the project unless its purpose and target are clear. Do not test on real hardware or real user data without approval.
- Before changing kernel, boot, memory allocator, driver, storage format, or low-level unsafe code, inspect the call path and relevant tests carefully; if evidence is insufficient, report the risk instead of guessing.

## Performance and UX
- Measure or establish a baseline before claiming memory, storage, speed, or binary-size improvements.
- Prefer eliminating unnecessary allocations, duplicated state, repeated work, and unneeded assets over speculative micro-optimizations.
- Do not reduce safety, reliability, readability, or accessibility just to optimize metrics.
- For UI changes, preserve the project's visual language and verify keyboard focus, readable contrast, clear states, and sensible scaling where applicable.
- Do not add a new dependency unless necessary. Explain its cost and alternatives first.

## Scope control
- Do not build an entire OS from scratch in one session.
- First complete an audit, then choose a few high-impact, low-risk improvements supported by the code.
- Prefer up to 6 bounded work items per run. Each work item must have a clear acceptance check.
- If a task requires missing hardware, unavailable dependencies, network access, elevated permissions, or a product decision, document the blocker and continue only with safe independent work.
- Do not fabricate missing requirements. Ask a concise question or record the assumption when the choice materially affects behavior.

## Completion report
At the end, provide:
1. Summary of actual changes.
2. Files changed.
3. Exact verification commands and pass/fail outcomes.
4. Security/performance/UX observations with evidence and any unverified claims clearly labeled.
5. Known issues and the next best task.
