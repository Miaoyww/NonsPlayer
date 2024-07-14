using System.Timers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;
using NonsPlayer.Core.Utils;
using NonsPlayer.Helpers;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class HitokotoCardViewModel
{
    private Timer timer;
    [ObservableProperty] private string hitokoto;

    public async Task<string?> GetHitokotoAsync(string category = "a")
    {
        try
        {
            string url = $"https://v1.hitokoto.cn/?c={category}";
            var result = JObject.Parse(await HttpUtils.HttpGetAsync(url));
            return result["hitokoto"]!.ToString();
        }
        catch (Exception)
        {
            try
            {
                string url = $"https://international.v1.hitokoto.cn/?c={category}";
                var result = JObject.Parse(await HttpUtils.HttpGetAsync(url));
                return result["hitokoto"]!.ToString();
            }
            catch (Exception)
            {
                return null;
            }
        }
    }

    public HitokotoCardViewModel()
    {
        timer = new Timer(60000);

        timer.Elapsed += OnTimedEvent;
        timer.AutoReset = true;
        timer.Start();
        Hitokoto = "Loading...";
        OnTimedEvent(null, null);
    }

    private async void OnTimedEvent(object? source, ElapsedEventArgs? e)
    {
        var text = await GetHitokotoAsync();
        if (text is null)
        {
            return;
        }

        ServiceHelper.DispatcherQueue.TryEnqueue(() => Hitokoto = text);
    }
}