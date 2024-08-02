using System.Runtime.InteropServices;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using NonsPlayer.Activation;
using NonsPlayer.AMLL.Components.ViewModels;
using NonsPlayer.AMLL.ViewModels;
using NonsPlayer.Cache;
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
using Serilog;
using Windows.Graphics.Display;
using UnhandledExceptionEventArgs = Microsoft.UI.Xaml.UnhandledExceptionEventArgs;
using WinRT;

namespace NonsPlayer;

// To learn more about WinUI 3, see https://docs.microsoft.com/windows/apps/winui/winui3/.
public partial class App : Application
{
    public App()
    {
        InitializeComponent();
        ServiceHelper.DispatcherQueue = DispatcherQueue.GetForCurrentThread();
        Host = Microsoft.Extensions.Hosting.Host.CreateDefaultBuilder().UseContentRoot(AppContext.BaseDirectory)
            .ConfigureServices((context, services) =>
            {
                Log.Logger = new LoggerConfiguration().WriteTo.File(
                        path: ConfigManager.Instance.Settings.Log,
                        outputTemplate:
                        "[{Timestamp:HH:mm:ss.fff}] [{Level:u4}] [{SourceContext}]: {Message}{Exception}{NewLine}")
                    .Enrich.FromLogContext()
                    .CreateLogger();
                Log.Information($"System: {Environment.OSVersion}");
                Log.Information($"Command Line: {Environment.CommandLine}");
                services.AddLogging(c => c.AddSerilog(Log.Logger));
                // Default Activation Handler
                services.AddTransient<ActivationHandler<LaunchActivatedEventArgs>, DefaultActivationHandler>();

                // Other Activation Handlers
                services.AddTransient<IActivationHandler, AppNotificationActivationHandler>();

                // Services
                services.AddSingleton<IAppNotificationService, AppNotificationService>();
                services.AddSingleton<IThemeSelectorService, ThemeSelectorService>();
                services.AddSingleton<IActivationService, ActivationService>();
                services.AddSingleton<IPageService, PageService>();
                services.AddSingleton<INavigationService, NavigationService>();
                services.AddSingleton<IFileService, FileService>();
                services.AddSingleton<VersionService>();
                services.AddSingleton<UpdateService>();
                services.AddSingleton<UpdateClient>();
                services.AddSingleton<ExceptionService>();
                services.AddSingleton<ControlService>();
                services.AddSingleton<PlayCounterService>();
                services.AddSingleton<SMTCService>();
                services.AddSingleton<RadioService>();
                services.AddSingleton<CacheService>();

                #region Views and ViewModels

                services.AddTransient<PlaylistDetailViewModel>();
                services.AddTransient<PlaylistDetailPage>();
                services.AddTransient<ExploreViewModel>();
                services.AddTransient<ExplorePage>();
                services.AddTransient<HomeViewModel>();
                services.AddTransient<HomePage>();
                services.AddTransient<SettingsViewModel>();
                services.AddTransient<SettingsPage>();
                services.AddTransient<ShellPage>();
                services.AddTransient<ShellViewModel>();
                services.AddTransient<PersonalCenterPage>();
                services.AddTransient<PersonalCenterViewModel>();
                services.AddTransient<PersonalLibaryPage>();
                services.AddTransient<PersonalLibaryViewModel>();
                services.AddTransient<LoginViewModel>();
                services.AddTransient<LoginPage>();
                services.AddTransient<SearchViewModel>();
                services.AddTransient<SearchPage>();
                services.AddTransient<LyricViewModel>();
                services.AddTransient<LyricPage>();
                services.AddTransient<ArtistPage>();
                services.AddTransient<ArtistViewModel>();
                services.AddTransient<AlbumPage>();
                services.AddTransient<AlbumViewModel>();
                services.AddTransient<UpdatePage>();
                services.AddTransient<UpdateViewModel>();
                services.AddTransient<LocalPage>();
                services.AddTransient<LocalViewModel>();
                services.AddTransient<TestPage>();
                services.AddTransient<TestViewModel>();
                services.AddTransient<AdapterManagerPage>();
                services.AddTransient<AdapterManagerViewModel>();

                #endregion

                #region Components Views and ViewModels

                services.AddTransient<PlaylistMusicItemCardViewModel>();
                services.AddTransient<PlaylistCardViewModel>();
                services.AddTransient<PlayerBarViewModel>();
                services.AddTransient<PlayQueueBarViewModel>();
                services.AddTransient<PlayQueueItemCardViewModel>();
                services.AddTransient<BestMusicCardViewModel>();
                services.AddTransient<BestArtistCardViewModel>();
                services.AddTransient<MusicListBarViewModel>();
                services.AddTransient<AdapterCardViewModel>();
                services.AddTransient<GreetingsCardViewModel>();
                services.AddTransient<HitokotoCardViewModel>();
                services.AddTransient<RecentlyPlayCardViewModel>();
                services.AddTransient<TodayDurationCardViewModel>();
                services.AddTransient<TotalListenCardViewModel>();
                services.AddTransient<RecentlyPlayItemCardViewModel>();
                services.AddTransient<RecommendedPlaylistCardViewModel>();
                services.AddTransient<RadioCardViewModel>();
                services.AddTransient<LoginCardViewModel>();
                services.AddTransient<FavoriteSongCardViewModel>();

                #endregion


                // AMLL
                services.AddTransient<AMLLViewModel>();
                services.AddTransient<LyricCardViewModel>();
            }).Build();

        GetService<IAppNotificationService>().Initialize();
        Log.Information($"Start loading config, current config path:{ConfigManager.Instance.Settings.ConfigFilePath}");
        ConfigManager.Instance.Load();
        AdapterService.Instance.AdapterLoadFailed += OnAdapterLoadFailed;
        AdapterService.Instance.AdapterLoading += OnAdapterLoading;
        Log.Information($"Start loading adapters, current adapter path:{ConfigManager.Instance.Settings.AdapterPath}");
        AdapterService.Instance.Init();
        Log.Information("Try log in adapter");
        foreach (var key in ConfigManager.Instance.Settings.AdapterAccountTokens.Keys)
        {
            Log.Information($"{key} try to log in");
            var adapter = AdapterService.Instance.GetAdapter(key);
            if (adapter != null)
            {
                adapter.Account.GetAccount()
                    .LoginByTokenAsync(ConfigManager.Instance.Settings.AdapterAccountTokens[key]);
                Log.Information($"{key} log in successfully");
            }
        }

        UnhandledException += App_UnhandledException;
        Log.Information($"Start loading player counter");
        GetService<PlayCounterService>().Init(ConfigManager.Instance.Settings.TotalPlayCount,
            ConfigManager.Instance.Settings.TodayPlayDuration);
        Log.Information($"Start loading SMTC service");
        GetService<SMTCService>().Init();
    }

    private void OnAdapterLoading(string param, Exception? exception)
    {
        Log.Information("Loading adapter from {path}", param);
    }

    private void OnAdapterLoadFailed(string name, Exception? exception)
    {
        Log.Error($"Failed loading adapter {name}: {exception}");
        ExceptionService.Instance.Throw($"Failed load adapter {name}: {exception}");
    }

    // The .NET Generic Host provides dependency injection, configuration, logging, and other services.
    // https://docs.microsoft.com/dotnet/core/extensions/generic-host
    // https://docs.microsoft.com/dotnet/core/extensions/dependency-injection
    // https://docs.microsoft.com/dotnet/core/extensions/configuration
    // https://docs.microsoft.com/dotnet/core/extensions/logging
    public IHost Host { get; }

    public static WindowEx MainWindow { get; } = new MainWindow();

    public static T GetService<T>()
        where T : class
    {
        if ((Current as App)!.Host.Services.GetService(typeof(T)) is not T service)
            throw new ArgumentException($"{typeof(T)} needs to be registered in ConfigureServices within App.xaml.cs.");

        return service;
    }

    private void App_UnhandledException(object sender, UnhandledExceptionEventArgs e)
    {
        Log.Error($"Exception threw: {e.Exception}");
        ExceptionService.Instance.Throw(e.Exception);
    }

    public static ILogger<T> GetLogger<T>()
    {
        return GetService<ILogger<T>>();
    }

    protected override async void OnLaunched(LaunchActivatedEventArgs args)
    {
        base.OnLaunched(args);
        // App.GetService<IAppNotificationService>().Show(string.Format("AppNotificationSamplePayload".GetLocalized(), AppContext.BaseDirectory));
        await GetService<IActivationService>().ActivateAsync(args);
    }
}

public static class WindowExtensions
{
    public static IntPtr GetWindowHandle(this Window window)
    {
        return window is null
            ? throw new ArgumentNullException(nameof(window))
            : window.As<IWindowNative>().WindowHandle;
    }

    // https://www.sharpgis.net/post/Using-the-CWin32-code-generator-to-enhance-your-WinUI-3-app
    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("EECDBF0E-BAE9-4CB6-A68E-9598E1CB57BB")]
    internal interface IWindowNative
    {
        IntPtr WindowHandle { get; }
    }
}