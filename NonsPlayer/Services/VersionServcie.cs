using System.Reflection;
using CommunityToolkit.Mvvm.ComponentModel;

namespace NonsPlayer.Services;

public partial class VersionService : ObservableObject
{
    [ObservableProperty] private Version currentVersion;
    [ObservableProperty] private string currentVersionDescription = string.Empty;


    public VersionService()
    {
        GetCurrentVersion();
        GetVersionDescription();
    }

    public Version GetCurrentVersion()
    {
        if (CurrentVersion == null) CurrentVersion = Assembly.GetExecutingAssembly().GetName().Version!;

        return CurrentVersion;
    }

    public string GetVersionDescription()
    {
        if (CurrentVersion == null) GetCurrentVersion();
        if (CurrentVersionDescription == string.Empty)
            CurrentVersionDescription = $"v{CurrentVersion.Major}.{CurrentVersion.Minor}.{CurrentVersion.Build}";

        return CurrentVersionDescription;
    }
}