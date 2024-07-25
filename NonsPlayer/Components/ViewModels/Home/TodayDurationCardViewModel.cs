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
        Player.Instance.PositionChangedHandle += PositionChangedHandle;
    }

    private void PositionChangedHandle(TimeSpan time)
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
}