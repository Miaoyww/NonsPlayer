using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class TotalListenCardViewModel
{
    public PlayCounterService PlayCounterService = App.GetService<PlayCounterService>();
    [ObservableProperty] private string totalListen;

    public TotalListenCardViewModel()
    {
        PlayCounterService.CounterChanged += OnCounterChanged;
        OnCounterChanged();
    }

    private void OnCounterChanged()
    {
        TotalListen = PlayCounterService.TotalPlayCount.ToString();
    }
}