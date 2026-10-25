param([ValidateSet('check','test','dev','build','clippy','probe')][string]$Task = 'dev', [string[]]$ExtraArgs = @(), [ValidateSet('network_probe','media_probe','qr_probe','library_probe','search_probe','lyrics_probe','local_ncm_probe')][string]$Example = 'network_probe')
$ErrorActionPreference = 'Stop'
$nonsRoot = Split-Path $PSScriptRoot -Parent
Push-Location (Join-Path $nonsRoot 'app')
try {
    if ($Task -in @('dev','build')) {
        & pnpm tauri $Task @ExtraArgs
    } else {
        Push-Location 'src-tauri'
        try {
            if ($Task -eq 'probe') { & cargo run --example $Example @ExtraArgs }
            else { & cargo $Task @ExtraArgs }
        } finally { Pop-Location }
    }
    exit $LASTEXITCODE
} finally { Pop-Location }
