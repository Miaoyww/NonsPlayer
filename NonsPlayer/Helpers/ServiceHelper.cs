using Microsoft.UI.Dispatching;
using NonsPlayer.Contracts.Services;
using NonsPlayer.ViewModels;
using NonsPlayer.Views;

namespace NonsPlayer.Helpers;

public static class ServiceHelper
{
    public static DispatcherQueue DispatcherQueue { get; set; }

    public static INavigationService NavigationService => ShellViewModel.OutNavigationService;
}