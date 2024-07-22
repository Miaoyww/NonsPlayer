using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Services;

namespace NonsPlayer.ViewModels;

public partial class ExploreViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private IMusic[] dailyRecommendedPlaylist;

    public async void OnNavigatedTo(object parameter)
    {
        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Common);
        foreach (var item in adapters)
        {
            var music = await item.Common.GetDailyRecommended();
            if (music != null)
            {
                DailyRecommendedPlaylist = music;
                break;
            }
        }
    }

    public void OnNavigatedFrom()
    {
    }
}