using System.Collections.ObjectModel;
using System.Reflection;
using System.Runtime.InteropServices;
using Windows.ApplicationModel;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.Updater.Update;
using System.Diagnostics;

namespace NonsPlayer.ViewModels;

public partial class SettingsViewModel : ObservableRecipient, INavigationAware
{
    private readonly UpdateService _updateService = App.GetService<UpdateService>();
    private readonly VersionService _versionService = App.GetService<VersionService>();
    [ObservableProperty] private string versionDescription;
    [ObservableProperty] private string currentVersion;
    [ObservableProperty] private string playListLoadOffset;
    [ObservableProperty] private string trackCount;
    [ObservableProperty] private string recommendedCount;
    [ObservableProperty] private string volumeStep;

    public SettingsViewModel()
    {
        RefreshInfo();
    }

    public void RefreshInfo()
    {
        VersionDescription = _versionService.CurrentVersionDescription;
        CurrentVersion = _versionService.CurrentVersion;
        PlayListLoadOffset = AppConfig.Instance.AppSettings.PlaylistLoadOffset.ToString();
        TrackCount = AppConfig.Instance.AppSettings.PlaylistTrackCount.ToString();
        RecommendedCount = AppConfig.Instance.AppSettings.RecommendedPlaylistCount.ToString();
        VolumeStep = AppConfig.Instance.AppSettings.VolumeStep.ToString();
    }

    public void Save()
    {
        AppConfig.Instance.AppSettings.PlaylistLoadOffset = Convert.ToInt32(PlayListLoadOffset);
        AppConfig.Instance.AppSettings.PlaylistTrackCount = Convert.ToInt32(TrackCount);
        AppConfig.Instance.AppSettings.RecommendedPlaylistCount = Convert.ToInt32(RecommendedCount);
        AppConfig.Instance.AppSettings.VolumeStep = Convert.ToInt32(VolumeStep);
        AppConfig.Instance.Save();
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

    public void OnNavigatedTo(object parameter)
    {
    }

    public void OnNavigatedFrom()
    {
        Save();
    }
}