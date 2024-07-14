using System.Timers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class GreetingsCardViewModel
{
    private Timer timer;
    [ObservableProperty] private string timeString;

    public GreetingsCardViewModel()
    {
        timer = new Timer(3000);

        timer.Elapsed += OnTimedEvent;
        timer.AutoReset = true;
        timer.Start();
        TimeString = DateTime.Now.ToString("HH:mm");
    }


    private void OnTimedEvent(object source, ElapsedEventArgs e)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() => TimeString = DateTime.Now.ToString("HH:mm"));
    }
}