using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using System.Diagnostics;
using System.Runtime.InteropServices;
using Windows.System;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml;
using NonsPlayer.Services;
using NonsPlayer.Updater.Github;
using NonsPlayer.Updater.Update;
using NonsPlayer.Views.Pages;

namespace NonsPlayer.ViewModels;

public partial class UpdateViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private ReleaseVersion latestVersion;
    [ObservableProperty] private string newVersion;
    public VersionService VersionService = App.GetService<VersionService>();

    public void OnNavigatedTo(object parameter)
    {
        LatestVersion = (ReleaseVersion)parameter;
        NewVersion = "v" + latestVersion.Version;
    }

    public void OnNavigatedFrom()
    {
    }

    [RelayCommand]
    public async Task ReferToUrl(string tag)
    {
        try
        {
            var url = tag switch
            {
                "github" => LatestVersion.ReleasePageURL,
                "portable" => LatestVersion.Portable,
                _ => null,
            };
            // _logger.LogInformation("Open url: {url}", url);
            if (Uri.TryCreate(url, UriKind.RelativeOrAbsolute, out var uri))
            {
                await Launcher.LaunchUriAsync(uri);
            }
        }

        catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
        }
    }
}