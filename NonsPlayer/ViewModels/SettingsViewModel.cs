﻿using System.Reflection;
using System.Runtime.InteropServices;
using Windows.ApplicationModel;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Core.Nons.Player;
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
    
}