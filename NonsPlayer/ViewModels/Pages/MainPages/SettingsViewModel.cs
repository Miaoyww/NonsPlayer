using System.Collections.ObjectModel;
using System.Reflection;
using System.Runtime.InteropServices;
using Windows.ApplicationModel;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.Updater.Update;
using System.Diagnostics;

namespace NonsPlayer.ViewModels;

public partial class SettingsViewModel : ObservableRecipient
{
    private readonly UpdateService _updateService = App.GetService<UpdateService>();
    private readonly VersionService _versionService = App.GetService<VersionService>();
    [ObservableProperty] public string versionDescription;


    public SettingsViewModel(IThemeSelectorService themeSelectorService)
    {
        VersionDescription = _versionService.CurrentVersionDescription;
    }

    [RelayCommand]
    private async Task Test()
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(TestViewModel)?.FullName);
    }

    [RelayCommand]
    public async void CheckUpdate()
    {
        var release =
            await _updateService.CheckUpdateAsync(_versionService.CurrentVersion, RuntimeInformation.OSArchitecture);
        if (release != null) ServiceHelper.NavigationService.NavigateTo(typeof(UpdateViewModel).FullName, release);
    }

    [RelayCommand]
    public void OpenAdapterSetting()
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(AdapterManagerViewModel)?.FullName);
    }

    [RelayCommand]
    public void OpenAdapterFolder()
    {
        Process.Start("explorer.exe", ConfigManager.Instance.Settings.ConfigFilePath);
    }
}