# AetherOS extended QEMU keyboard test
# Tests: printable chars, backspace, tab, enter, shift, caps lock
param(
    [int]$BootWaitMs = 4000,
    [int]$KeyDelayMs = 200,
    [string]$Accel = "whpx"
)

$ErrorActionPreference = "Stop"

$repo = "D:\aether_kernel"
$bootImg = Join-Path $repo "target\x86_64-aether_kernel\debug\bootimage-aether_kernel.bin"
$serialLog = Join-Path $repo "qemu_serial_extended.log"
$qemu = "D:\Program Files\qemu\qemu-system-x86_64.exe"
$monitorPort = 5556

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

    function Send-Key($key) {
        Write-Host "[TEST] sendkey $key"
        Send-Monitor "sendkey $key"
    }

    # Give monitor a moment to emit banner
    Send-Monitor ""

    # Test 1: Basic typing "hello"
    Write-Host "[TEST] === Test 1: Basic typing ==="
    foreach ($k in @("h","e","l","l","o")) { Send-Key $k }
    Start-Sleep -Milliseconds 500

    # Test 2: Backspace (delete "lo" -> "hel")
    Write-Host "[TEST] === Test 2: Backspace ==="
    Send-Key "ret"  # Enter to see line
    Start-Sleep -Milliseconds 300
    foreach ($k in @("h","e","l","l","o")) { Send-Key $k }
    Start-Sleep -Milliseconds 300
    Send-Key "backspace"
    Send-Key "backspace"
    Start-Sleep -Milliseconds 300

    # Test 3: Tab
    Write-Host "[TEST] === Test 3: Tab ==="
    Send-Key "ret"
    Start-Sleep -Milliseconds 300
    Send-Key "tab"
    Send-Key "t"
    Send-Key "a"
    Send-Key "b"
    Start-Sleep -Milliseconds 300

    # Test 4: Shift + letters (uppercase)
    Write-Host "[TEST] === Test 4: Shift ==="
    Send-Key "ret"
    Start-Sleep -Milliseconds 300
    Send-Monitor "sendkey shift-h"
    Send-Monitor "sendkey shift-e"
    Send-Monitor "sendkey shift-l"
    Send-Monitor "sendkey shift-l"
    Send-Monitor "sendkey shift-o"
    Start-Sleep -Milliseconds 500

    # Test 5: Caps Lock
    Write-Host "[TEST] === Test 5: Caps Lock ==="
    Send-Key "ret"
    Start-Sleep -Milliseconds 300
    Send-Key "capslock"
    Start-Sleep -Milliseconds 200
    foreach ($k in @("c","a","p","s")) { Send-Key $k }
    Start-Sleep -Milliseconds 300
    Send-Key "capslock"
    Start-Sleep -Milliseconds 200
    foreach ($k in @("o","f","f")) { Send-Key $k }
    Start-Sleep -Milliseconds 500

    # Test 6: Enter key (newline)
    Write-Host "[TEST] === Test 6: Enter ==="
    Send-Key "ret"
    Start-Sleep -Milliseconds 300
    foreach ($k in @("n","e","w","l","i","n","e")) { Send-Key $k }
    Send-Key "ret"
    Start-Sleep -Milliseconds 500

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