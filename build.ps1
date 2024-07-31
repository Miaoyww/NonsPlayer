param(
    [string] $Architecture = "x64",
    [string] $Version = "1.0.0",
    [switch] $Dev
)

$ErrorActionPreference = "Stop";

if ($Dev) {
    dotnet publish NonsPlayer -c Release -r "win10-$Architecture" -o "build/NonsPlayer/app-$Version" -p:Platform=$Architecture -p:DefineConstants=DEV -p:PublishReadyToRun=true -p:Version=$Version  -p:UseRidGraph=true;
}
else {
    dotnet publish NonsPlayer -c Release -r "win10-$Architecture" -o "build/NonsPlayer/app-$Version" -p:Platform=$Architecture -p:PublishReadyToRun=true -p:Version=$Version -p:UseRidGraph=true;
}

$env:Path += ';C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\amd64\';

msbuild NonsPlayer.Launcher "-property:Configuration=Release;Platform=$Architecture;OutDir=$(Resolve-Path "build/NonsPlayer/")";

Add-Content "build/NonsPlayer/version.ini" -Value "exe_path=app-$Version\NonsPlayer.exe";

Remove-Item "build/NonsPlayer/NonsPlayer.pdb" -Force;