using System.Runtime.InteropServices;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using NonsPlayer.Activation;
using NonsPlayer.AMLL.Components.ViewModels;
using NonsPlayer.AMLL.ViewModels;
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
                // Default Activation Handler
                services.AddTransient<ActivationHandler<LaunchActivatedEventArgs>, DefaultActivationHandler>();

                // Other Activation Handlers
                services.AddTransient<IActivationHandler, AppNotificationActivationHandler>();

                // Services
                services.AddSingleton<IAppNotificationService, AppNotificationService>();
                services.AddSingleton<ILocalSettingsService, LocalSettingsService>();
                services.AddSingleton<IThemeSelectorService, ThemeSelectorService>();
                services.AddSingleton<IActivationService, ActivationService>();
                services.AddSingleton<IPageService, PageService>();
                services.AddSingleton<INavigationService, NavigationService>();
                services.AddSingleton<VersionService>();
                services.AddSingleton<UpdateService>();
                services.AddSingleton<UpdateClient>();
                services.AddSingleton<ExceptionService>();
                services.AddSingleton<ControlService>();

                // Core Services
                services.AddSingleton<IFileService, FileService>();

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
                services.AddTransient<RecommendedPlaylistCardViewModel>();
                services.AddTransient<UserPlaylistCardViewModel>();
                services.AddTransient<PlayerBarViewModel>();
                services.AddTransient<UserPlaylistBarViewModel>();
                services.AddTransient<PlayQueueBarViewModel>();
                services.AddTransient<PlayQueueItemCardViewModel>();
                services.AddTransient<BestMusicCardViewModel>();
                services.AddTransient<BestArtistCardViewModel>();
                services.AddTransient<MusicListBarViewModel>();
                services.AddTransient<AdapterCardViewModel>();
                #endregion


                // AMLL
                services.AddTransient<AMLLViewModel>();
                services.AddTransient<LyricCardViewModel>();

                // Configuration
                services.Configure<LocalSettingsOptions>(
                    context.Configuration.GetSection(nameof(LocalSettingsOptions)));
            }).Build();
        
        GetService<IAppNotificationService>().Initialize();
        ConfigManager.Instance.LoadConfigAsync();
        AdapterService.Instance.LoadAdapters("C:\\Users\\miaom\\Desktop\\Plugins");
        UnhandledException += App_UnhandledException;
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
        // TODO: Log and handle exceptions as appropriate.
        // https://docs.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.application.unhandledexception.
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