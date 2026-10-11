param([ValidateSet('check','test','dev','build','clippy','probe')][string]$Task = 'dev', [string[]]$ExtraArgs = @(), [ValidateSet('network_probe','media_probe','qr_probe','library_probe','search_probe','lyrics_probe','local_ncm_probe')][string]$Example = 'network_probe')
$ErrorActionPreference = 'Stop'
$nonsRoot = Split-Path $PSScriptRoot -Parent
$nonsGst = Join-Path $nonsRoot '.local\gstreamer'
if (Test-Path -LiteralPath (Join-Path $nonsGst 'VERSION')) {
    $env:PATH = (Join-Path $nonsGst 'bin') + ';' + $env:PATH
    $env:GST_PLUGIN_PATH = Join-Path $nonsGst 'lib\gstreamer-1.0'
    $env:GST_PLUGIN_SCANNER = Join-Path $nonsGst 'libexec\gstreamer-1.0\gst-plugin-scanner.exe'
    foreach ($nonsDependency in @{'GLIB_2_0'='glib-2.0-0'; 'GOBJECT_2_0'='gobject-2.0-0'; 'GIO_2_0'='gio-2.0-0'; 'GSTREAMER_1_0'='gstreamer-1.0-0'; 'GSTREAMER_BASE_1_0'='gstbase-1.0-0'; 'GSTREAMER_CONTROLLER_1_0'='gstcontroller-1.0-0'}.GetEnumerator()) {
        Set-Item ('Env:SYSTEM_DEPS_' + $nonsDependency.Key + '_NO_PKG_CONFIG') '1'
        Set-Item ('Env:SYSTEM_DEPS_' + $nonsDependency.Key + '_LIB') $nonsDependency.Value
        Set-Item ('Env:SYSTEM_DEPS_' + $nonsDependency.Key + '_SEARCH_NATIVE') (Join-Path $nonsGst 'lib')
    }
}
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
