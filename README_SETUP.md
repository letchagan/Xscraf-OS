# Install and run the Aether OS OpenCode agent

These files are designed for the existing Aether OS repository, expected in `D:\aether_kernel`. If your repository lives elsewhere, use its actual path.

## 1. Back up / inspect your current work
Open PowerShell and run:

```powershell
cd D:\aether_kernel
git status --short
```

If Git says this is not a repository, do not initialize/reset anything automatically; make a manual backup of the project first. If you have uncommitted work, keep it. The agent instructions tell OpenCode not to discard it.

## 2. Copy the pack into the repository root
Extract the ZIP. Copy these items into `D:\aether_kernel` while preserving the `.opencode` folder structure:

```text
D:\aether_kernel\AGENTS.md
D:\aether_kernel\AETHER_OS_OVERNIGHT_TASK.md
D:\aether_kernel\README_SETUP.md
D:\aether_kernel\.opencode\agents\aether-builder.md
D:\aether_kernel\.opencode\commands\aether-overnight.md
```

If `AGENTS.md`, `.opencode\agents\aether-builder.md`, or `.opencode\commands\aether-overnight.md` already exists, inspect it and merge instead of blindly overwriting your own instructions.

PowerShell copy example (run from the extracted pack directory; change the path if needed):

```powershell
Copy-Item .\AGENTS.md D:\aether_kernel\AGENTS.md
Copy-Item .\AETHER_OS_OVERNIGHT_TASK.md D:\aether_kernel\AETHER_OS_OVERNIGHT_TASK.md
Copy-Item .\README_SETUP.md D:\aether_kernel\README_SETUP.md
New-Item -ItemType Directory -Force D:\aether_kernel\.opencode\agents | Out-Null
New-Item -ItemType Directory -Force D:\aether_kernel\.opencode\commands | Out-Null
Copy-Item .\.opencode\agents\aether-builder.md D:\aether_kernel\.opencode\agents\aether-builder.md
Copy-Item .\.opencode\commands\aether-overnight.md D:\aether_kernel\.opencode\commands\aether-overnight.md
```

## 3. Start OpenCode in the project directory

```powershell
cd D:\aether_kernel
opencode
```

If `opencode` is not recognized, use the working `opencode.cmd` path from your installation. Do not run an incompatible `opencode.exe` directly.

## 4. Select a free model
Inside OpenCode, enter:

```text
/models
```

Select **Nemotron 3 Ultra Free** if it appears and is available in your session. If unavailable or it performs poorly on your codebase, test **Ling 3.1 Flash Free** next. Model availability and names may change; choose from the actual selector rather than hard-coding an unverified model ID.

## 5. Run the custom agent
Inside OpenCode:

1. Enter `/agents` and select `aether-builder`, or press `Tab` to cycle through primary agents until it is selected.
2. Run the command:

```text
/aether-overnight
```

If the command is not listed, check that the file is exactly at `.opencode\commands\aether-overnight.md` beneath the repository root, then restart OpenCode.

## 6. Before leaving it unattended
First run it while you are present and review the plan and first changes. Make sure tests actually run on your machine. The files limit work to six items per run, require progress notes, and prohibit destructive/privileged operations; these instructions reduce risk but cannot guarantee the model will never loop or make a mistake.

For later continuation, start a new session in the same directory and ask it to read `PROGRESS.md`, inspect the current diff, and continue only with the next documented task. Do not let it auto-commit or run privileged commands unattended.
