using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class TodayDurationCardViewModel
{
    [ObservableProperty] private string _duration;
    private PlayCounterService PlayCounterService => App.GetService<PlayCounterService>();

    public TodayDurationCardViewModel()
    {
        ChangeTime();
        Player.Instance.PositionChanged += OnPositionChanged;
    }

    private void OnPositionChanged(TimeSpan time)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            ChangeTime();
        });
    }

    public void ChangeTime()
    {
        Duration = PlayCounterService.TodayPlayDuration.ToString(@"mm") + "M";
    }

    public void TodayDurationCard_OnUnloaded(object sender, RoutedEventArgs e)
    {
        try
        {
            Player.Instance.PositionChanged -= OnPositionChanged;
        }
        catch
        {
            // ignore
        }
    }
}