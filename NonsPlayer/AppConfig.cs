using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using NonsPlayer.Activation;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using NonsPlayer.Services;
using NonsPlayer.Updater.Update;
using NonsPlayer.ViewModels;
using NonsPlayer.Views;
using NonsPlayer.Views.Pages;
using WinRT;
using UnhandledExceptionEventArgs = Microsoft.UI.Xaml.UnhandledExceptionEventArgs;

namespace NonsPlayer;

internal static class AppConfig
{
    public static string LogFile { get; private set; }

    static AppConfig()
    {
        Initialize();
    }

    public static string? IgnoreVersion { get; set; }
    public static string? AppVersion { get; set; }

    private static void Initialize()
    {
#if DEBUG
    AppVersion = "0.4.0";
#else
        AppVersion = typeof(AppConfig).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion;
#endif
    }
}