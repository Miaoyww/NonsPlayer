using System.Reflection;
using System.Runtime.InteropServices;
using Windows.ApplicationModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.Updater.Update;

namespace NonsPlayer.ViewModels;

public partial class SettingsViewModel : ObservableRecipient
{
    private readonly UpdateService _updateService = App.GetService<UpdateService>();
    private readonly VersionService _versionService = App.GetService<VersionService>();

    [ObservableProperty] public string versionDescription;

    public SettingsViewModel(IThemeSelectorService themeSelectorService)
    {
        VersionDescription = GetVersionDescription();
    }

    [RelayCommand]
    private void Test()
    {
        Account.Instance.LogOut();
        App.MainWindow.Close();
    }

    [RelayCommand]
    public async void CheckUpdate()
    {
        var release =
            await _updateService.CheckUpdateAsync(_versionService.CurrentVersion, RuntimeInformation.OSArchitecture);
        if (release != null) ServiceHelper.NavigationService.NavigateTo(typeof(UpdateViewModel).FullName, release);
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
}