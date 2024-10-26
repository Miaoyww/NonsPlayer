using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Forms;
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

    [ObservableProperty] private Visibility isProgressBarVisible;

    [ObservableProperty] private Visibility isProgressTextVisible;

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

    private async void UpdatePage_OnLoaded(object sender, RoutedEventArgs e)
    {
        await GetReleaseAsync();
    }

    private async Task GetReleaseAsync()
    {
        try
        {
            if (ViewModel.LatestVersion is null)
            {
                ViewModel.LatestVersion = await _updateClient.GetLatestVersionAsync(false,
                    RuntimeInformation.OSArchitecture);
            }

            LatestRelease = ViewModel.LatestVersion;
            _timer.Start();
            await _updateService.PrepareForUpdateAsync(LatestRelease);
            UpdateProgressState();
            _timer.Stop();

            await ShowGithubReleaseAsync(ViewModel.LatestVersion.Version);
        }
        catch (HttpRequestException ex)
        {
            ExceptionService.Instance.Throw($"Cannot get latest release: {ex.Message}");
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
        }
    }

    private void UpdatePage_OnUnloaded(object sender, RoutedEventArgs e)
    {
        _timer.Stop();
        _updateService.Stop();
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
                BorderMarkdown.Visibility = Visibility.Visible;
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
            ExceptionService.Instance.Throw($"Cannot get github release: {ex.Message}");
            if (!tag.Contains("dev"))
            {
                webview.Source = new Uri($"https://github.com/Miaoyww/NonsPlayer/releases/tag/{tag}");
                BorderMarkdown.Visibility = Visibility.Visible;
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


    private void Restart()
    {
        try
        {
            var baseDir = new DirectoryInfo(AppContext.BaseDirectory).Parent?.FullName;
            var exe = Path.Join(baseDir, "NonsPlayer.exe");
            if (File.Exists(exe))
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = exe,
                    WorkingDirectory = baseDir
                });
                AppConfig.Instance.IgnoreVersion = null;
                Environment.Exit(0);
            }
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
        }
        finally
        {
            ButtonUpdate.IsEnabled = true;
            ButtonRemindLatter.IsEnabled = true;
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
        AppConfig.Instance.IgnoreVersion = LatestRelease?.Version;
        ServiceHelper.NavigationService.GoBack();
    }


    [RelayCommand]
    private async Task UpdateNowAsync()
    {
        try
        {
            ErrorMessage = null;
            ButtonUpdate.IsEnabled = false;
            ButtonRemindLatter.IsEnabled = false;

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
            ButtonUpdate.IsEnabled = true;
            ButtonRemindLatter.IsEnabled = true;
        }
    }

    [RelayCommand]
    public async Task ReferToUrl(string tag)
    {
        try
        {
            var url = tag switch
            {
                "github" => ViewModel.LatestVersion.ReleasePageURL,
                "portable" => ViewModel.LatestVersion.Portable,
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

    public void _timer_Tick(DispatcherQueueTimer sender, object args)
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

    public void UpdateProgressState()
    {
        if (_updateService.State is UpdateService.UpdateState.Preparing)
        {
            IsProgressTextVisible = Visibility.Collapsed;
            IsProgressBarVisible = Visibility.Visible;
            ProgressBarUpdate.IsIndeterminate = true;
        }

        if (_updateService.State is UpdateService.UpdateState.Pending)
        {
            IsProgressTextVisible = Visibility.Visible;
            IsProgressBarVisible = Visibility.Visible;
            ProgressBarUpdate.IsIndeterminate = false;
            UpdateProgressValue();
        }


        if (_updateService.State is UpdateService.UpdateState.Downloading)
        {
            ButtonUpdate.IsEnabled = false;
            ButtonRemindLatter.IsEnabled = false;
            IsProgressBarVisible = Visibility.Visible;
            IsProgressTextVisible = Visibility.Visible;
            ProgressBarUpdate.IsIndeterminate = false;
            UpdateProgressValue();
        }

        if (_updateService.State is UpdateService.UpdateState.Moving)
        {
            ButtonUpdate.IsEnabled = false;
            ButtonRemindLatter.IsEnabled = false;
            IsProgressBarVisible = Visibility.Visible;
            IsProgressTextVisible = Visibility.Visible;
            ProgressBarUpdate.IsIndeterminate = true;
            UpdateProgressValue();
        }

        if (_updateService.State is UpdateService.UpdateState.Finish)
        {
            IsProgressTextVisible = Visibility.Collapsed;
            ProgressBarUpdate.IsIndeterminate = false;
            ProgressBarUpdate.Value = 100;
        }

        if (_updateService.State is UpdateService.UpdateState.Stop)
        {
            IsProgressTextVisible = Visibility.Collapsed;
            IsProgressBarVisible = Visibility.Collapsed;
            ErrorMessage = null;
            ButtonUpdate.IsEnabled = true;
            ButtonRemindLatter.IsEnabled = true;
        }

        if (_updateService.State is UpdateService.UpdateState.Error)
        {
            IsProgressTextVisible = Visibility.Collapsed;
            IsProgressBarVisible = Visibility.Collapsed;
            ErrorMessage = _updateService.ErrorMessage;
            ButtonUpdate.IsEnabled = true;
            ButtonRemindLatter.IsEnabled = true;
        }

        if (_updateService.State is UpdateService.UpdateState.NotSupport)
        {
            IsProgressTextVisible = Visibility.Collapsed;
            IsProgressBarVisible = Visibility.Collapsed;
            ErrorMessage = _updateService.ErrorMessage;
            ButtonUpdate.IsEnabled = false;
            ButtonRemindLatter.IsEnabled = true;
        }
    }

    public void UpdateProgressValue()
    {
        const double mb = 1 << 20;
        ProgressBytesText =
            $"{_updateService.Progress_BytesDownloaded / mb:F2}/{_updateService.Progress_BytesToDownload / mb:F2} MB";
        var progress = (double)_updateService.Progress_BytesDownloaded / _updateService.Progress_BytesToDownload;
        ProgressPercentText = $"{progress:P1}";
        ProgressBarUpdate.Value = progress * 100;
    }

    private void Button_RemindLatter_PointerEntered(object sender, PointerRoutedEventArgs e)
    {
        ButtonRemindLatter.Opacity = 1;
    }

    private void Button_RemindLatter_PointerExited(object sender, PointerRoutedEventArgs e)
    {
        ButtonRemindLatter.Opacity = 0;
    }
}