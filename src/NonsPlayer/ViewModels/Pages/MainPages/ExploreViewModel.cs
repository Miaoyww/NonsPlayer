using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Services;

namespace NonsPlayer.ViewModels;

public partial class ExploreViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private IMusic[] dailyRecommendedPlaylist;
    private ILogger logger = App.GetLogger<ExploreViewModel>();

    public async void OnNavigatedTo(object parameter)
    {
        if (!string.IsNullOrEmpty(ConfigManager.Instance.Settings.DefaultAdapter))
        {
            var adapter = AdapterService.Instance.GetAdapter(ConfigManager.Instance.Settings.DefaultAdapter);
            if (adapter != null)
            {
                logger.LogInformation($"Explore got adapter {adapter.GetMetadata().DisplayPlatform}");
                var music = await adapter.Common.GetDailyRecommended();
                if (music != null)
                {
                    DailyRecommendedPlaylist = music;
                    return;
                }
            }
            else
            {
                logger.LogInformation($"Explore couldn't get an adapter");  
            }
        }

        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Common);
        if (adapters != null)
        {
            if (adapters.Length != 0)
            {
                logger.LogInformation($"Explore got adapter {adapters[0].GetMetadata().DisplayPlatform}");
                ConfigManager.Instance.Settings.DefaultAdapter = adapters[0].GetMetadata().Name;
                var music = await adapters[0].Common.GetDailyRecommended();
                if (music != null)
                {
                    DailyRecommendedPlaylist = music;
                }
            }
        }
    }

    public void OnNavigatedFrom()
    {
    }
}