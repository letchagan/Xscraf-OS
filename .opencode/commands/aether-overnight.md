---
description: Run a bounded Aether OS security, correctness, UX, and performance improvement session
agent: aether-builder
---

Read `AGENTS.md` and `AETHER_OS_OVERNIGHT_TASK.md`. Inspect the current repository and Git state first. Do not assume its architecture or stack.

Run one bounded improvement session:
1. Audit the existing implementation and identify the top risks and opportunities with file/path evidence.
2. Pick up to six small work items, prioritizing confirmed security and correctness problems, then low-risk UI/UX and RAM/storage improvements.
3. Implement and verify one item at a time. Update `PROGRESS.md` after every item and keep user changes intact.
4. Stop repeating any failed command after two identical attempts; stop and diagnose repetitive model output; do not make speculative changes to fill time.
5. Do not add AI integrations, do not perform destructive/privileged actions, and do not commit.
6. Finish with changes, exact test outcomes, unverified risks, blockers, and the next recommended task.

Start with inspection and a short plan, then proceed without asking for approval for ordinary, reversible code edits. Ask before any risky or irreversible action.
