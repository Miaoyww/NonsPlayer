using System.Runtime.InteropServices;
using Windows.System;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.Web.WebView2.Core;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.Updater.Github;
using NonsPlayer.Updater.Update;

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
}