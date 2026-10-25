param(
    [ValidateSet('init', 'check', 'build')][string]$Task = 'build',
    [string]$JavaHome = $env:JAVA_HOME
)
$ErrorActionPreference = 'Stop'
$nonsRoot = Split-Path $PSScriptRoot -Parent
$nonsAndroid = Join-Path $nonsRoot '.local/android'
$nonsNdkVersion = '27.2.12479018'
if (-not $JavaHome) {
    $nonsJdks = Get-ChildItem 'C:/Program Files/Java' -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like 'jdk-21*' } | Sort-Object Name -Descending
    if ($nonsJdks) { $JavaHome = $nonsJdks[0].FullName }
}
if (-not $JavaHome -or -not (Test-Path -LiteralPath (Join-Path $JavaHome 'bin/java.exe'))) {
    throw 'Provide a JDK via JAVA_HOME or -JavaHome. Android Studio is not required.'
}
$env:JAVA_HOME = $JavaHome
$env:ANDROID_HOME = Join-Path $nonsAndroid 'sdk'
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:NDK_HOME = Join-Path $env:ANDROID_HOME "ndk/$nonsNdkVersion"
$env:GRADLE_USER_HOME = Join-Path $nonsAndroid 'gradle'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:RUSTFLAGS = "$env:RUSTFLAGS -C link-arg=-Wl,-z,max-page-size=16384".Trim()
$env:PATH = (Join-Path $JavaHome 'bin') + ';' + (Join-Path $env:ANDROID_HOME 'platform-tools') + ';' + $env:PATH
Push-Location $nonsRoot
try {
    if ($Task -eq 'init') {
        & pnpm --dir app tauri android init --ci
        exit $LASTEXITCODE
    }
    if ($Task -eq 'check') {
        $nonsClang = Join-Path $env:NDK_HOME 'toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android24-clang.cmd'
        $env:CC_aarch64_linux_android = $nonsClang
        $env:AR_aarch64_linux_android = Join-Path $env:NDK_HOME 'toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-ar.exe'
        $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $nonsClang
        & cargo check --manifest-path app/src-tauri/Cargo.toml --lib --target aarch64-linux-android
    } else {
        & pnpm --dir app tauri android build --debug --apk --target aarch64 --ci
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $nonsApk = Join-Path $nonsRoot 'app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk'
        & python scripts/verify-android-apk.py $nonsApk
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $nonsSignature = & (Join-Path $env:ANDROID_HOME 'build-tools/37.0.0/apksigner.bat') verify --verbose --print-certs $nonsApk
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $nonsSignature | Write-Host
        # Default verification starts at minSdk 24 and can skip the V1 scheme.
        # Verify at API 23 as well to explicitly test the JAR signature.
        $nonsJarSignature = & (Join-Path $env:ANDROID_HOME 'build-tools/37.0.0/apksigner.bat') verify --verbose --min-sdk-version 23 $nonsApk 2>$null
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        foreach ($nonsScheme in @('v1', 'v2', 'v3')) {
            if (-not ($nonsJarSignature -match "Verified using $nonsScheme scheme .*: true")) {
                throw "APK is missing required $nonsScheme signature"
            }
        }
        & (Join-Path $env:ANDROID_HOME 'build-tools/37.0.0/zipalign.exe') -c -P 16 4 $nonsApk
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $nonsVersion = (Get-Content -Raw 'app/package.json' | ConvertFrom-Json).version
        $nonsOutput = Join-Path $nonsAndroid "Nons-$nonsVersion-android-arm64-debug.apk"
        Copy-Item -LiteralPath $nonsApk -Destination $nonsOutput
        Copy-Item -LiteralPath $nonsApk -Destination (Join-Path $nonsAndroid "Nons-$nonsVersion-android-arm64-debug-compat.apk")
        $nonsHash = [System.Security.Cryptography.SHA256]::Create()
        $nonsStream = [System.IO.File]::OpenRead($nonsOutput)
        try {
            $nonsDigest = ([BitConverter]::ToString($nonsHash.ComputeHash($nonsStream))).Replace('-', '').ToLowerInvariant()
        } finally {
            $nonsStream.Dispose()
            $nonsHash.Dispose()
        }
        Set-Content -LiteralPath "$nonsOutput.sha256" -Value "$nonsDigest  $(Split-Path $nonsOutput -Leaf)" -Encoding ascii
        Write-Host "APK: $nonsOutput"
    }
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
