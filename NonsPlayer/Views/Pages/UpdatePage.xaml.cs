using System.Diagnostics;
using System.Runtime.InteropServices;
using Windows.System;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.Web.WebView2.Core;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Updater.Github;
using NonsPlayer.Updater.Update;
using NonsPlayer.ViewModels;
using DispatcherQueueTimer = Microsoft.UI.Dispatching.DispatcherQueueTimer;

namespace NonsPlayer.Views.Pages;

[INotifyPropertyChanged]
public sealed partial class UpdatePage : Page
{
    private readonly DispatcherQueueTimer _timer;
    private readonly UpdateClient _updateClient = App.GetService<UpdateClient>();
    private readonly UpdateService _updateService = App.GetService<UpdateService>();

    [ObservableProperty] private string? errorMessage;

    [ObservableProperty] private bool isProgressBarVisible;

    [ObservableProperty] private bool isProgressTextVisible;

    [ObservableProperty] private ReleaseVersion latestRelease;

    [ObservableProperty] private string progressBytesText;

    [ObservableProperty] private string progressCountText;

    [ObservableProperty] private string progressPercentText;

    [ObservableProperty] private string progressSpeedText;

    public UpdatePage()
    {
        ViewModel = App.GetService<UpdateViewModel>();
        InitializeComponent();
        _timer = DispatcherQueue.CreateTimer();
        _timer.Interval = TimeSpan.FromMilliseconds(100);
        _timer.Tick += _timer_Tick;
    }

    public UpdateViewModel ViewModel { get; }

    private void UpdatePage_OnLoaded(object sender, RoutedEventArgs e)
    {
        _timer.Stop();
        _updateService.Stop();
    }

    private void UpdatePage_OnUnloaded(object sender, RoutedEventArgs e)
    {
        _timer.Stop();
        _updateService.Stop();
    }

    private async Task GetReleaseAsync()
    {
        try
        {
            if (ViewModel.LatestVersion is null)
                ViewModel.LatestVersion = await _updateClient.GetLatestVersionAsync(false,
                    RuntimeInformation.OSArchitecture);

            await ShowGithubReleaseAsync(ViewModel.LatestVersion.Version);

            LatestRelease =
                await _updateClient.GetLatestVersionAsync(false,
                    RuntimeInformation.OSArchitecture);
            _timer.Start();
            await _updateService.PrepareForUpdateAsync(LatestRelease);
            UpdateProgressState();
            _timer.Stop();
        }
        catch (HttpRequestException ex)
        {
            // _logger.LogWarning("Cannot get latest release: {error}", ex.Message);
        }
        catch (Exception ex)
        {
            // _logger.LogWarning(ex, "Get release");
        }
    }


    private async Task ShowGithubReleaseAsync(string tag)
    {
        try
        {
            var release = await _updateClient.GetGithubReleaseAsync(tag);
            if (release != null)
            {
                var markdown = $"""
                                # {release.Name}

                                > Update at {release.PublishedAt.LocalDateTime:yyyy-MM-dd HH:mm:ss}

                                {release.Body}

                                """;
                var html = await _updateClient.RenderGithubMarkdownAsync(markdown);
                var cssFile = Path.Combine(AppContext.BaseDirectory, @"Assets\CSS\github-markdown-dark.css");
                var css = "";
                if (File.Exists(cssFile)) css = await File.ReadAllTextAsync(cssFile);

                html = $$"""
                         <!DOCTYPE html>
                         <html>
                         <head>
                         <base target="_blank">
                         <meta name="color-scheme" content="dark">
                         {{(string.IsNullOrWhiteSpace(css) ? """<link href="https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/5.2.0/github-markdown-dark.min.css" type="text/css" rel="stylesheet" />""" : "")}}
                         <style>
                         body::-webkit-scrollbar {display: none;}
                         {{css}}
                         </style>
                         </head>
                         <body style="background-color: #404040; margin: 24px;">
                         <article class="markdown-body" style="background-color: #404040;">
                         {{html}}
                         </article>
                         </body>
                         </html>
                         """;
                await webview.EnsureCoreWebView2Async();
                webview.NavigateToString(html);
                Border_Markdown.Visibility = Visibility.Visible;
                webview.CoreWebView2.Settings.AreDevToolsEnabled = false;
                webview.CoreWebView2.Settings.AreDefaultContextMenusEnabled = false;
                webview.CoreWebView2.Settings.AreBrowserAcceleratorKeysEnabled = false;
                webview.CoreWebView2.NewWindowRequested += async (s, e) =>
                {
                    try
                    {
                        e.Handled = true;
                        await Launcher.LaunchUriAsync(new Uri(e.Uri));
                    }
                    catch
                    {
                    }
                };
            }
        }
        catch (Exception ex)
        {
            // _logger.LogWarning("Cannot get github release: {error}", ex.Message);
            if (!tag.Contains("dev"))
            {
                webview.Source = new Uri($"https://github.com/Scighost/Starward/releases/tag/{tag}");
                Border_Markdown.Visibility = Visibility.Visible;
                await webview.EnsureCoreWebView2Async();
                webview.CoreWebView2.Profile.PreferredColorScheme =
                    CoreWebView2PreferredColorScheme.Dark;
                webview.CoreWebView2.NewWindowRequested += async (s, e) =>
                {
                    try
                    {
                        e.Handled = true;
                        await Launcher.LaunchUriAsync(new Uri(e.Uri));
                    }
                    catch
                    {
                    }
                };
            }
        }
    }


    [RelayCommand]
    private void RemindMeLatter()
    {
        ServiceHelper.NavigationService.GoBack();
    }


    [RelayCommand]
    private void IgnoreThisVersion()
    {
        // AppConfig.IgnoreVersion = NewVersion?.Version;
        ServiceHelper.NavigationService.GoBack();
    }


    [RelayCommand]
    private async Task UpdateNowAsync()
    {
        try
        {
            // ErrorMessage = null;
            Button_Update.IsEnabled = false;
            Button_RemindLatter.IsEnabled = false;

            if (LatestRelease is null)
                LatestRelease = await _updateClient.GetLatestVersionAsync(false, RuntimeInformation.OSArchitecture);

            _timer.Start();
            while (_updateService.State is UpdateService.UpdateState.Preparing) await Task.Delay(100);

            if (_updateService.State is not UpdateService.UpdateState.Pending)
                await _updateService.PrepareForUpdateAsync(LatestRelease);

            if (_updateService.State is UpdateService.UpdateState.Pending) _updateService.Start();

            _timer.Start();
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
            // _logger.LogError(ex, "Update now");
            Button_Update.IsEnabled = true;
            Button_RemindLatter.IsEnabled = true;
        }
    }


    private void UpdateProgressState()
    {
        if (_updateService.State is UpdateService.UpdateState.Preparing)
        {
            // IsProgressTextVisible = false;
            // IsProgressBarVisible = true;
            // ProgresBar_Update.IsIndeterminate = true;
        }

        if (_updateService.State is UpdateService.UpdateState.Pending)
            // IsProgressTextVisible = true;
            // IsProgressBarVisible = true;
            // ProgresBar_Update.IsIndeterminate = false;
            UpdateProgressValue();

        if (_updateService.State is UpdateService.UpdateState.Downloading)
        {
            Button_Update.IsEnabled = false;
            Button_RemindLatter.IsEnabled = false;
            // IsProgressBarVisible = true;
            // IsProgressTextVisible = true;
            // ProgresBar_Update.IsIndeterminate = false;
            UpdateProgressValue();
        }

        if (_updateService.State is UpdateService.UpdateState.Moving)
        {
            Button_Update.IsEnabled = false;
            Button_RemindLatter.IsEnabled = false;
            // IsProgressBarVisible = true;
            // IsProgressTextVisible = true;
            // ProgresBar_Update.IsIndeterminate = true;
            UpdateProgressValue();
        }

        if (_updateService.State is UpdateService.UpdateState.Finish)
        {
            // IsProgressTextVisible = false;
            // ProgresBar_Update.IsIndeterminate = false;
            // ProgresBar_Update.Value = 100;
        }

        if (_updateService.State is UpdateService.UpdateState.Stop)
        {
            // IsProgressTextVisible = false;
            // IsProgressBarVisible = false;
            // ErrorMessage = null;
            Button_Update.IsEnabled = true;
            Button_RemindLatter.IsEnabled = true;
        }

        if (_updateService.State is UpdateService.UpdateState.Error)
        {
            // IsProgressTextVisible = false;
            // IsProgressBarVisible = false;
            // ErrorMessage = _updateService.ErrorMessage;
            Button_Update.IsEnabled = true;
            Button_RemindLatter.IsEnabled = true;
        }

        if (_updateService.State is UpdateService.UpdateState.NotSupport)
        {
            // IsProgressTextVisible = false;
            // IsProgressBarVisible = false;
            // ErrorMessage = _updateService.ErrorMessage;
            Button_Update.IsEnabled = false;
            Button_RemindLatter.IsEnabled = true;
        }
    }


    private void UpdateProgressValue()
    {
        const double mb = 1 << 20;
        // ProgressBytesText =
        //     $"{_updateService.Progress_BytesDownloaded / mb:F2}/{_updateService.Progress_BytesToDownload / mb:F2} MB";
        // ProgressCountText =
        //     $"{_updateService.Progress_FileCountDownloaded}/{_updateService.Progress_FileCountToDownload}";
        // var progress = (double)_updateService.Progress_BytesDownloaded / _updateService.Progress_BytesToDownload;
        // ProgressPercentText = $"{progress:P1}";
        // ProgresBar_Update.Value = progress * 100;
    }


    private void _timer_Tick(DispatcherQueueTimer sender, object args)
    {
        try
        {
            UpdateProgressState();
            if (_updateService.State is UpdateService.UpdateState.Finish)
            {
                _timer.Stop();
                Restart();
            }

            if (_updateService.State is UpdateService.UpdateState.Stop or UpdateService.UpdateState.Error
                or UpdateService.UpdateState.NotSupport)
                _timer.Stop();
        }
        catch (Exception ex)
        {
            // _logger.LogError(ex, "Update progress");
        }
    }


    private void Restart()
    {
        try
        {
            var baseDir = new DirectoryInfo(AppContext.BaseDirectory).Parent?.FullName;
            var exe = Path.Join(baseDir, "Starward.exe");
            if (File.Exists(exe))
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = exe,
                    WorkingDirectory = baseDir
                });
                // AppConfig.IgnoreVersion = null;
                Environment.Exit(0);
            }
        }
        catch (Exception ex)
        {
            // _logger.LogWarning(ex, "Restart");
            // ErrorMessage = ex.Message;
        }
        finally
        {
            Button_Update.IsEnabled = true;
            Button_RemindLatter.IsEnabled = true;
        }
    }


    private void Button_RemindLatter_PointerEntered(object sender, PointerRoutedEventArgs e)
    {
        Button_RemindLatter.Opacity = 1;
    }

    private void Button_RemindLatter_PointerExited(object sender, PointerRoutedEventArgs e)
    {
        Button_RemindLatter.Opacity = 0;
    }
}