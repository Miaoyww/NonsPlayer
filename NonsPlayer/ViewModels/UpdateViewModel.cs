using Windows.System;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Services;
using NonsPlayer.Services;
using NonsPlayer.Updater.Github;

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
                _ => null
            };
            // _logger.LogInformation("Open url: {url}", url);
            if (Uri.TryCreate(url, UriKind.RelativeOrAbsolute, out var uri)) await Launcher.LaunchUriAsync(uri);
        }

        catch (Exception e)
        {
            ExceptionService.Instance.Throw(e);
        }
    }
}