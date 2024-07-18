using System.Diagnostics;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.Views;

public sealed partial class PlayBar : UserControl
{
    public PlayBar()
    {
        ViewModel = App.GetService<PlayerBarViewModel>();
        InitializeComponent();
        Player.Instance.PositionChangedHandle += OnPositionChanged;
    }

    private ManipulationStartedRoutedEventArgs? _slidingEventArgs = null;
    private TimeSpan StartingTimeSpan = TimeSpan.Zero;
    private bool _isSliding = false;

    public PlayerBarViewModel ViewModel { get; }

    public event EventHandler OnPlayQueueBarOpenHandler;

    [RelayCommand]
    public void OpenLyric()
    {
        // if (ServiceHelper.NavigationService.Frame.Content.GetType().ToString() == typeof(LyricPage)?.FullName)
        // {
        //     ServiceHelper.NavigationService.NavigateTo(ServiceHelper.NavigationService.LastPage, isBack: true);
        //     return;
        // }
        ServiceHelper.NavigationService.NavigateTo(typeof(LyricViewModel)?.FullName);
    }

    [RelayCommand]
    public void OpenPlayQueueBar()
    {
        OnPlayQueueBarOpenHandler?.Invoke(this, EventArgs.Empty);
    }

    [RelayCommand]
    private async void LikeMusic()
    {
        if (MusicStateModel.Instance.CurrentMusic == null) return;

        if (MusicStateModel.Instance.CurrentMusic.IsEmpty) return;
        // var code = await FavoritePlaylistService.Instance.LikeAsync(MusicStateModel.Instance.CurrentMusic.Id);
        // if (code != 200)
        // {
        //     string content;
        //     switch (code)
        //     {
        //         case 301:
        //             content = "请登录后再试";
        //             break;
        //         case 400:
        //             content = "请检查网络后再试";
        //             break;
        //         default:
        //             content = $"出现了错误 {code}";
        //             break;
        //     }
        //
        //     var dialog = new ContentDialog
        //     {
        //         XamlRoot = XamlRoot,
        //         Style = Application.Current.Resources["DefaultContentDialogStyle"] as Style,
        //         Title = "错误",
        //         PrimaryButtonText = "知道了",
        //         CloseButtonText = "取消",
        //         DefaultButton = ContentDialogButton.Primary,
        //         Content = content
        //     };
        //     await dialog.ShowAsync();
        // }
    }

    public void OnPositionChanged(TimeSpan position)
    {
        try
        {
            if (Player.Instance.CurrentMusic != null)
            {
                if (Player.Instance.CurrentMusic.IsEmpty) return;
                if (!_isSliding)
                {
                    DispatcherQueue.TryEnqueue(() =>
                    {
                        MusicStateModel.Instance.Position = Player.Instance.CurrentReader.CurrentTime.TotalSeconds;
                        CurrentTimeSlider.Value = Player.Instance.CurrentReader.CurrentTime.TotalSeconds;
                    });
                }
            }
        }
        catch
        {
            // ignore
        }
    }

    private void CurrentTimeSlider_OnManipulationCompleted(object sender, ManipulationCompletedRoutedEventArgs e)
    {
        _slidingEventArgs = null;
        var value = TimeSpan.FromSeconds(CurrentTimeSlider.Value);
        if (Math.Abs((value - StartingTimeSpan).TotalMilliseconds) > 250d)
        {
            PlayerService.Seek(value);
        }

        _isSliding = false;
    }

    private void CurrentTimeSlider_OnManipulationStarting(object sender, ManipulationStartingRoutedEventArgs e)
    {
        var value = TimeSpan.FromSeconds(CurrentTimeSlider.Value);
        StartingTimeSpan = value;
        PlayerService.Seek(value);
    }

    private void CurrentTimeSlider_OnManipulationStarted(object sender, ManipulationStartedRoutedEventArgs e)
    {
        _isSliding = true;
        _slidingEventArgs = e;
    }
}