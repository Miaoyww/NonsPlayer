using System.Reflection;
using System.Windows.Input;
using Windows.ApplicationModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public class SettingsViewModel : ObservableRecipient
{
    private void Test()
    {

    }
    // private void Test()
    // {
    //     Account.Instance.LogOut();
    //     App.MainWindow.Close();
    // }

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

    public ICommand SwitchThemeCommand { get; }

    public ICommand TestButtonClickCommand { get; }

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

            version = new Version(packageVersion.Major, packageVersion.Minor, packageVersion.Build,
                packageVersion.Revision);
        }
        else
        {
            version = Assembly.GetExecutingAssembly().GetName().Version!;
        }

        return $"v{version.Major}.{version.Minor}.{version.Build}";
    }

    #endregion
}