param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$nonsPluginRoot = Split-Path $PSScriptRoot -Parent
$nonsPluginApp = Join-Path $nonsPluginRoot 'app'
$nonsPluginTarget = Join-Path $nonsPluginApp 'src-tauri/target/plugin-probe'
& node (Join-Path $nonsPluginRoot 'scripts/build-plugin.mjs') settings-fixture (Join-Path $nonsPluginApp 'src-tauri/target/plugin-fixtures/settings-fixture')
if ($LASTEXITCODE -ne 0) { throw 'Configuration fixture build failed.' }
if (-not $SkipBuild) {
    Push-Location $nonsPluginApp
    $nonsPluginPreviousTarget = $env:CARGO_TARGET_DIR
    try { $env:CARGO_TARGET_DIR = $nonsPluginTarget; & pnpm tauri build --debug --no-bundle --features plugin-probe; if ($LASTEXITCODE -ne 0) { throw 'Plugin probe build failed.' } }
    finally { $env:CARGO_TARGET_DIR = $nonsPluginPreviousTarget; Pop-Location }
}
$nonsPluginStdout = [IO.Path]::GetTempFileName()
$nonsPluginStderr = [IO.Path]::GetTempFileName()
$nonsPluginProcess = $null
try {
    $nonsPluginProcess = Start-Process -FilePath (Join-Path $nonsPluginTarget 'debug/Nons.exe') -WorkingDirectory $nonsPluginApp -WindowStyle Hidden -PassThru -RedirectStandardOutput $nonsPluginStdout -RedirectStandardError $nonsPluginStderr
    if (-not $nonsPluginProcess.WaitForExit(100000)) { $nonsPluginProcess.Kill($true); throw 'Plugin probe timed out.' }
    $nonsPluginOutput = Get-Content -LiteralPath $nonsPluginStdout -Raw
    Write-Output $nonsPluginOutput
    if ($nonsPluginProcess.ExitCode -ne 0 -or $nonsPluginOutput -notmatch 'uninstall removes storage, page and navigation contributions: passed') {
        Write-Output (Get-Content -LiteralPath $nonsPluginStderr -Raw)
        throw 'Plugin probe failed.'
    }
} finally {
    # Check every resolved cleanup target before recursive deletion, using one shell.
    $nonsPluginTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    $nonsPluginTargets = @($nonsPluginStdout, $nonsPluginStderr)
    if ($null -ne $nonsPluginProcess) { $nonsPluginTargets += (Join-Path $nonsPluginTempRoot "nons-plugin-probe-$($nonsPluginProcess.Id)") }
    foreach ($nonsPluginTarget in $nonsPluginTargets) {
        $nonsPluginResolved = [IO.Path]::GetFullPath($nonsPluginTarget)
        if (-not $nonsPluginResolved.StartsWith($nonsPluginTempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Probe cleanup target escaped the temporary directory.' }
        if (Test-Path -LiteralPath $nonsPluginResolved) { Remove-Item -LiteralPath $nonsPluginResolved -Recurse -Force }
    }
}
