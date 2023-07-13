using System.Diagnostics;
using System.Reflection;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Framework.Model;
using NonsPlayer.Helpers;
using Windows.ApplicationModel;
using NonsPlayer.Services;
using CommunityToolkit.Maui.Alerts;
using CommunityToolkit.Maui.Core;
using Microsoft.UI.Xaml.Controls;

namespace NonsPlayer.ViewModels;

public class SettingsViewModel : ObservableRecipient
{
    private void Test()
    {

    }

    #region 66

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

    #endregion
}