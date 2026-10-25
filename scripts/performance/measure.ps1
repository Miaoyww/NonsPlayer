param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [int]$Runs = 5,
    [int]$SampleSeconds = 20,
    [int]$AudioRuns = 5,
    [string]$OutputDirectory = ''
)
$ErrorActionPreference = 'Stop'
$nonsRepo = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$nonsExe = (Resolve-Path -LiteralPath $Executable).Path
if ($Runs -lt 1 -or $Runs -gt 20 -or $SampleSeconds -lt 5 -or $SampleSeconds -gt 120 -or $AudioRuns -lt 3 -or $AudioRuns -gt 30) { throw 'Invalid measurement bounds.' }
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $nonsRepo ('.local/performance/' + (Get-Date -Format 'yyyyMMdd-HHmmss')) }
$nonsOutput = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $nonsOutput -Force | Out-Null
$nonsBinary = Join-Path $nonsOutput 'bin'
New-Item -ItemType Directory -Path $nonsBinary | Out-Null
Copy-Item -LiteralPath $nonsExe -Destination (Join-Path $nonsBinary 'Nons.exe')
# A --no-bundle build does not stage Tauri resources. Copy only current resources;
# never inherit retired adapters or stale plugins from the Cargo output directory.
foreach ($nonsResource in @('bundled-adapters', 'bundled-plugins')) {
    Copy-Item -LiteralPath (Join-Path $nonsRepo ('app/src-tauri/' + $nonsResource)) -Destination (Join-Path $nonsBinary $nonsResource) -Recurse
}
$nonsExe = Join-Path $nonsBinary 'Nons.exe'
$nonsCpu = Get-CimInstance Win32_Processor
$nonsOs = Get-CimInstance Win32_OperatingSystem
$nonsHardware = Get-CimInstance Win32_ComputerSystem
$nonsMetadata = [ordered]@{
    recordedAt = (Get-Date).ToString('o'); executable = $nonsExe; sha256 = (Get-FileHash -LiteralPath $nonsExe -Algorithm SHA256).Hash
    commit = (& git -C $nonsRepo rev-parse HEAD); dirty = [bool](& git -C $nonsRepo status --porcelain)
    os = $nonsOs.Caption; osVersion = $nonsOs.Version; cpu = @($nonsCpu | ForEach-Object { $_.Name }); logicalProcessors = $nonsHardware.NumberOfLogicalProcessors
    ramBytes = $nonsHardware.TotalPhysicalMemory; gpu = @(Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name })
    soundDevices = @(Get-CimInstance Win32_SoundDevice | ForEach-Object { $_.Name })
    powerScheme = (& powercfg /getactivescheme | Out-String).Trim()
    playbackEngine = 'WebAudio'
    runs = $Runs; sampleSeconds = $SampleSeconds; audioRuns = $AudioRuns; sampleIntervalMs = 1000
    workload = 'Release performance-probe; new isolated profile per run; 48kHz stereo 16bit silent WAV; local LRC; no online audio'
}
$nonsMetadata | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $nonsOutput 'metadata.json') -Encoding utf8
$nonsPreviousRoot = $env:NONS_PERFORMANCE_ROOT
try {
    for ($nonsRun = 1; $nonsRun -le $Runs; $nonsRun++) {
        $nonsRunDirectory = Join-Path $nonsOutput ('run-' + $nonsRun.ToString('00'))
        New-Item -ItemType Directory -Path $nonsRunDirectory | Out-Null
        & python (Join-Path $PSScriptRoot 'fixtures.py') $nonsRunDirectory
        if ($LASTEXITCODE) { throw 'Fixture generation failed.' }
        @{ sampleSeconds = $SampleSeconds; audioRuns = $AudioRuns } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $nonsRunDirectory 'config.json') -Encoding utf8
        $env:NONS_PERFORMANCE_ROOT = $nonsRunDirectory
        $nonsLaunch = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        $nonsProcess = Start-Process -FilePath $nonsExe -WorkingDirectory (Split-Path $nonsExe -Parent) -WindowStyle Hidden -PassThru
        @{ launchEpochMs = $nonsLaunch; pid = $nonsProcess.Id } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $nonsRunDirectory 'launch.json') -Encoding utf8
        $nonsWatch = [Diagnostics.Stopwatch]::StartNew()
        $nonsWriter = [IO.StreamWriter]::new((Join-Path $nonsRunDirectory 'samples.jsonl'), $false, [Text.UTF8Encoding]::new($false))
        try {
            while (-not $nonsProcess.HasExited) {
                if ($nonsWatch.Elapsed.TotalSeconds -gt (180 + 6 * $SampleSeconds + 2 * $AudioRuns)) { $nonsProcess.Kill(); throw "Run $nonsRun timed out." }
                $nonsProcesses = @(Get-CimInstance Win32_Process)
                $nonsIds = [Collections.Generic.HashSet[int]]::new()
                [void]$nonsIds.Add($nonsProcess.Id)
                do {
                    $nonsChanged = $false
                    foreach ($nonsMember in $nonsProcesses) {
                        if ($nonsIds.Contains([int]$nonsMember.ParentProcessId) -and $nonsIds.Add([int]$nonsMember.ProcessId)) { $nonsChanged = $true }
                    }
                } while ($nonsChanged)
                $nonsRows = @(foreach ($nonsId in $nonsIds) {
                    try {
                        $nonsMember = Get-Process -Id $nonsId -ErrorAction Stop
                        @{ pid = $nonsId; cpuSeconds = $nonsMember.TotalProcessorTime.TotalSeconds; privateBytes = $nonsMember.PrivateMemorySize64; workingSetBytes = $nonsMember.WorkingSet64 }
                    } catch { }
                })
                $nonsWriter.WriteLine((@{ epochMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); processes = $nonsRows } | ConvertTo-Json -Depth 5 -Compress))
                $nonsWriter.Flush()
                Start-Sleep -Milliseconds 1000
                $nonsProcess.Refresh()
            }
        } finally { $nonsWriter.Dispose() }
        $nonsEvents = Join-Path $nonsRunDirectory 'events.jsonl'
        if (-not (Test-Path -LiteralPath $nonsEvents) -or -not (Select-String -LiteralPath $nonsEvents -Pattern '"event":"finished".*"success":true' -Quiet)) { throw "Run $nonsRun did not complete successfully. Inspect events and isolated logs." }
        Write-Output "Completed run $nonsRun/$Runs : $nonsRunDirectory"
    }
} finally { $env:NONS_PERFORMANCE_ROOT = $nonsPreviousRoot }
& python (Join-Path $PSScriptRoot 'report.py') $nonsOutput
if ($LASTEXITCODE) { throw 'Report generation failed.' }
Write-Output "Performance baseline: $nonsOutput"
