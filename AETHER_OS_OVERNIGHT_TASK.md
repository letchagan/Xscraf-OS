# Aether OS — Bounded Overnight Coding Task

## Goal
Improve the existing Aether OS project while preserving its current direction. Focus on security flaws, correctness, practical UI features/usability, and RAM/storage efficiency. Do not add any AI integration or AI-powered feature in this task.

## Phase 1 — Discover and baseline
- Read the README and project docs.
- Map the major directories and identify which parts are implemented, prototypes, generated files, or placeholders.
- Check the current Git status and inspect recent diffs before editing.
- Determine the real build, lint, test, and formatting commands from repository files.
- Run only safe, bounded baseline checks that the repository supports.
- Write findings into `PROGRESS.md`; separate confirmed findings from hypotheses.

## Phase 2 — Prioritize
Rank candidate items by:
1. Confirmed security or data-integrity defect.
2. Confirmed crash, incorrect state, input-handling, or reliability bug.
3. UI/UX issue that makes an existing feature hard to use or inaccessible.
4. RAM/storage overhead backed by code evidence or measurement.
5. Maintainability issue that is directly blocking correctness.

Prefer fixes with small diffs and a clear verification step. Do not introduce features or dependencies without a clear need.

## Phase 3 — Implement
Complete up to six bounded work items. For each one:
- Record the problem and relevant file/path evidence.
- Describe the intended fix briefly.
- Make one logical patch.
- Inspect `git diff` and confirm unrelated user changes remain untouched.
- Run the narrowest relevant test/check, then broader safe checks if practical.
- Record exact command and result in `PROGRESS.md`.

If there are no confirmed defects, document the audit and choose only a clearly useful, low-risk improvement. Do not manufacture tasks.

## Security requirements
- Review untrusted input, path handling, permissions, unsafe memory operations, error handling, process/network boundaries, and storage operations wherever relevant to the actual codebase.
- Preserve security checks. Do not silently change security-sensitive behavior to make the build pass.
- Do not use real credentials, production services, real user data, or unapproved hardware tests.
- Do not delete data, format disks/partitions, change boot records, run privileged commands, or change host security settings.

## UX requirements
Where actual UI code exists, prioritize clarity of system states, error messages, keyboard navigation, focus visibility, contrast, responsive layout, and consistency with existing design. Do not redesign unrelated screens.

## RAM and storage requirements
- Identify a likely cost or establish a baseline first.
- Prefer removing redundant work/state, unneeded asset duplication, or unnecessary allocations when supported by evidence.
- Keep reliability and security above micro-optimizations.
- Never claim a numeric improvement without a measurement.

## Stop conditions
Stop the current item and reassess if:
- The same command fails twice.
- The model repeats characters, lines, code, or a previous tool call.
- Two different approaches hit the same blocker.
- A change requires uncertain low-level behavior, privileged action, missing hardware, or destructive operations.
- The next change would be speculative or too broad to verify safely.

At most six work items this run. Do not commit changes. Leave a checkpoint that another session can continue.

## Final report
Include:
- Confirmed findings and their file/path locations.
- Work completed and files changed.
- Verification commands with actual results.
- What was not verified and why.
- Remaining high-priority risks.
- One precise recommended next task.
