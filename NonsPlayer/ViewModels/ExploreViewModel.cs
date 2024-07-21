using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Services;

namespace NonsPlayer.ViewModels;

public partial class ExploreViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private IPlaylist dailyRecommendedPlaylist;
    public ExploreViewModel()
    {
    }

    public void OnNavigatedTo(object parameter)
    {
        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Common);
        foreach (var item in adapters)
        {
            item.Common.GetRecommendedPlaylistAsync(10);
        }
    }

    public void OnNavigatedFrom()
    {
    }
}