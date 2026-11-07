# AetherOS automated QEMU keyboard test
# Injects keystrokes via the QEMU monitor (sendkey) and captures kernel serial output.
param(
    [int]$BootWaitMs = 4000,
    [int]$KeyDelayMs = 250,
    [string]$Accel = "whpx"
)

$ErrorActionPreference = "Stop"

$repo = "D:\aether_kernel"
$bootImg = Join-Path $repo "target\x86_64-aether_kernel\debug\bootimage-aether_kernel.bin"
$serialLog = Join-Path $repo "qemu_serial.log"
$qemu = "D:\Program Files\qemu\qemu-system-x86_64.exe"
$monitorPort = 5555

if (-not (Test-Path -LiteralPath $bootImg)) {
    Write-Error "Bootimage not found: $bootImg. Run 'cargo bootimage' first."
    exit 1
}

# Clean up stale serial log
if (Test-Path -LiteralPath $serialLog) { Remove-Item -LiteralPath $serialLog -Force }

# Kill any previous QEMU instance
Get-Process -Name "qemu-system-x86_64" -ErrorAction SilentlyContinue | Stop-Process -Force

$qemuArgs = @(
    "-m", "128M",
    "-accel", $Accel,
    "-drive", "format=raw,file=$bootImg",
    "-serial", "file:$serialLog",
    "-display", "none",
    "-no-reboot",
    "-monitor", "tcp:127.0.0.1:${monitorPort},server,nowait"
)

Write-Host "[TEST] Starting QEMU..."
$proc = Start-Process -FilePath $qemu -ArgumentList $qemuArgs -PassThru

try {
    Start-Sleep -Milliseconds $BootWaitMs

    # Connect to QEMU monitor
    Write-Host "[TEST] Connecting to QEMU monitor on port $monitorPort..."
    $tcp = New-Object System.Net.Sockets.TcpClient
    for ($i = 0; $i -lt 20 -and -not $tcp.Connected; $i++) {
        try { $tcp.Connect("127.0.0.1", $monitorPort) } catch { Start-Sleep -Milliseconds 300 }
    }
    if (-not $tcp.Connected) { throw "Could not connect to QEMU monitor" }

    $stream = $tcp.GetStream()
    $writer = New-Object System.IO.StreamWriter($stream)
    $writer.AutoFlush = $true

    function Send-Monitor($cmd) {
        $writer.WriteLine($cmd)
        Start-Sleep -Milliseconds $KeyDelayMs
    }

    # Give monitor a moment to emit banner
    Send-Monitor ""

    # Type a short sequence: h e l l o (space) a e t h e r
    foreach ($k in @("h","e","l","l","o","spc","a","e","t","h","e","r")) {
        Write-Host "[TEST] sendkey $k"
        Send-Monitor "sendkey $k"
    }

    # Give kernel time to process and log
    Start-Sleep -Milliseconds 1500
}
finally {
    if ($proc -and -not $proc.HasExited) {
        $proc | Stop-Process -Force
    }
}

Start-Sleep -Milliseconds 500

Write-Host "`n[TEST] ===== SERIAL LOG ====="
if (Test-Path -LiteralPath $serialLog) {
    Get-Content -LiteralPath $serialLog
} else {
    Get-PSDrive -Name D
}
