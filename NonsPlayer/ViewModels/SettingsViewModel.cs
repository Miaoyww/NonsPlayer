using System.Diagnostics;
using System.Reflection;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Framework.Model;
using NonsPlayer.Framework.Resources;
using NonsPlayer.Helpers;
using Windows.ApplicationModel;

namespace NonsPlayer.ViewModels;

public class SettingsViewModel : ObservableRecipient
{
    private readonly IThemeSelectorService _themeSelectorService;
    private ElementTheme _elementTheme;
    private string _versionDescription;

    public ElementTheme ElementTheme
    {
        get => _elementTheme;
        set => SetProperty(ref _elementTheme, value);
    }



    public string VersionDescription
    {
        get => _versionDescription;
        set => SetProperty(ref _versionDescription, value);
    }

    public ICommand SwitchThemeCommand
    {
        get;
    }
    public ICommand TestButtonClickCommand
    {
        get;
    }

    public SettingsViewModel(IThemeSelectorService themeSelectorService)
    {
        _versionDescription = GetVersionDescription();
        TestButtonClickCommand = new RelayCommand(Test);
    }

    private async void Test()
    {
        var b = (JObject)(await NonsApi.Api.Music.Detail(new long[] {2026787176}, ResEntry.nons))["songs"][0];
        var a = new Music(b);
        await a.GetLric();
        // await a.GetLric();
        // Debug.Print(a.Lyrics.Lrc[0].OriginalLyric);
    }
    private static string GetVersionDescription()
    {
        Version version;

        if (RuntimeHelper.IsMSIX)
        {
            var packageVersion = Package.Current.Id.Version;

            version = new(packageVersion.Major, packageVersion.Minor, packageVersion.Build, packageVersion.Revision);
        }
        else
        {
            version = Assembly.GetExecutingAssembly().GetName().Version!;
        }

        return $"v{version.Major}.{version.Minor}.{version.Build}";
    }
}