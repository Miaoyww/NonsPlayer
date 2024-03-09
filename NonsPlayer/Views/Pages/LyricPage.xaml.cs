using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class LyricPage : Page
{
    public LyricPage()
    {
        ViewModel = App.GetService<LyricViewModel>();
        InitializeComponent();
        Player.Instance.PositionChangedHandle += OnPositionChanged;
    }

    private ManipulationStartedRoutedEventArgs? _slidingEventArgs = null;
    private TimeSpan StartingTimeSpan = TimeSpan.Zero;
    private bool _isSliding = false;
    public LyricViewModel ViewModel { get; }

    public void OnPositionChanged(TimeSpan position)
    {
        if (Player.Instance.CurrentMusic.IsEmpty) return;
        if (!_isSliding)
        {
            DispatcherQueue.TryEnqueue(() =>
            {
                MusicStateModel.Instance.Position = Player.Instance.NPMediaFoundationReader.CurrentTime.TotalSeconds;
                CurrentTimeSlider.Value = Player.Instance.NPMediaFoundationReader.CurrentTime.TotalSeconds;
            });
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