﻿param(
    [string] $Architecture = "x64",
    [string] $Version = "1.0.0",
    [switch] $Dev
)

$ErrorActionPreference = "Stop";

$build = "build";
$nonsPlayer = "$build/NonsPlayer";
if ($Dev) {
    $metadata = "$build/metadata/dev";
    $package = "$build/release/package/dev";
    $separate = "$build/release/separate_files/dev";
}
else {
    $metadata = "$build/metadata/v1";
    $package = "$build/release/package";
    $separate = "$build/release/separate_files";
}

$null = New-Item -Path $package -ItemType Directory -Force;
$null = New-Item -Path $separate -ItemType Directory -Force;
$null = New-Item -Path $metadata -ItemType Directory -Force;

if (!(Get-Module -Name 7Zip4Powershell -ListAvailable)) {
    Install-Module -Name 7Zip4Powershell -Force;
}

$portableName = "NonsPlayer_$($Version)_$($Architecture).7z";
$portableFile = "$package/$portableName";

if (!(Test-Path $portableFile)) {
    Compress-7Zip -ArchiveFileName $portableName -Path $nonsplayer -OutputPath $package -CompressionLevel Ultra -PreserveDirectoryRoot;
}

$release = @{
    Version           = $Version
    Architecture      = $Architecture
    BuildTime         = Get-Date
    DisableAutoUpdate = $false
    InstallSize       = 0
    InstallHash       = $null
    Portable          = "https://github.com/Miaoyww/NonsPlayer/releases/download/$Version/$portableName"
    PortableSize      = (Get-Item $portableFile).Length
    PortableHash      = (Get-FileHash $portableFile).Hash
};

if ($Dev) {
    $release.Portable = "https://github.com/Miaoyww/NonsPlayer/releases/download/$Version/$portableName";
}

Out-File -Path "$metadata/version_preview_$Architecture.json" -InputObject (ConvertTo-Json $release);

$path = @{l = "Path"; e = { [System.IO.Path]::GetRelativePath($nonsplayer, $_.FullName) } };
$size = @{l = "Size"; e = { $_.Length } };
$hash = @{l = "Hash"; e = { (Get-FileHash $_).Hash } };

Out-File -Path "$metadata/release_preview_$Architecture.json" -InputObject (ConvertTo-Json $release);

foreach ($file in $release.SeparateFiles) {
    Move-Item -Path "$nonsplayer/$($file.Path)" -Destination "$separate/$($file.Hash)" -Force;
}
