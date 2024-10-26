using System.Reflection;
using CommunityToolkit.Mvvm.ComponentModel;

namespace NonsPlayer.Services;

public partial class VersionService : ObservableObject
{
    /// <summary>
    /// 不带v
    /// </summary>
    [ObservableProperty] private string currentVersion;
    
    [ObservableProperty] private string currentVersionDescription = string.Empty;
    /// <summary>
    /// 带v
    /// </summary>


    public VersionService()
    {
        GetCurrentVersion();
        GetVersionDescription();
    }

    public string GetCurrentVersion()
    {
        if (CurrentVersion == null) CurrentVersion = AppConfig.Instance.AppVersion!;

        return CurrentVersion;
    }

    public string GetVersionDescription()
    {
        if (CurrentVersion == null) GetCurrentVersion();
        if (CurrentVersionDescription == string.Empty)
            CurrentVersionDescription = $"v{CurrentVersion}";

        return CurrentVersionDescription;
    }
}