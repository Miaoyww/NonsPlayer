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
using NonsPlayer.Updater.Github;
using NonsPlayer.Updater.Update;
using NonsPlayer.Views.Pages;

namespace NonsPlayer.ViewModels;

public partial class UpdateViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private GithubVersion latestVersion;
    
    public void OnNavigatedTo(object parameter)
    {
        LatestVersion = (GithubVersion)parameter;
    }

    public void OnNavigatedFrom()
    {

    }
    
}