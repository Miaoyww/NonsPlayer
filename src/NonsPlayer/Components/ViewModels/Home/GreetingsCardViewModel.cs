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
    [ObservableProperty] private string greetings;

    public GreetingsCardViewModel()
    {
        Greetings = "Loading...";
        timer = new Timer(3000);

        timer.Elapsed += OnTimedEvent;
        timer.AutoReset = true;
        timer.Start();
        OnTimedEvent(null, null);
        TimeString = DateTime.Now.ToString("HH:mm");
    }


    private void OnTimedEvent(object source, ElapsedEventArgs e)
    {
        int hour = DateTime.Now.Hour;


        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            if (hour >= 5 && hour < 11)
            {
                Greetings = "GreetingsCard_Morning".GetLocalized();
            }
            else if (hour >= 11 && hour < 14)
            {
                Greetings = "GreetingsCard_Noon".GetLocalized();
            }
            else if (hour >= 14 && hour < 18)
            {
                Greetings = "GreetingsCard_AfterNoon".GetLocalized();
            }
            else
            {
                Greetings = "GreetingsCard_Night".GetLocalized();
            }

            TimeString = DateTime.Now.ToString("HH:mm");
        });
    }

    public void GreetingsCard_OnUnloaded(object sender, RoutedEventArgs e)
    {
        timer.Stop();
    }
}