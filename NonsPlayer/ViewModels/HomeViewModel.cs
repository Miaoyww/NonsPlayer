using System.Collections.ObjectModel;
using System.Drawing.Text;
using System.Text;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using SharpCompress.Common;
using static NonsPlayer.Core.Services.ControlFactory;

namespace NonsPlayer.ViewModels;

public partial class HomeViewModel : ObservableRecipient
{
    public INavigationService NavigationService;

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    [RelayCommand]
    public async void Test(string id)
    {
        var music = await AdapterService.Instance.GetAdapter("ncm").Music.GetMusicAsyncById(id);
        PlayQueue.Instance.Play(music);
    }

    public async void HomePage_OnLoaded(object sender, RoutedEventArgs e)
    {

        // var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Common);
        // List<IPlaylist> recommendedPlaylist = new();
        // foreach (var item in adapters)
        // {
        //     recommendedPlaylist.AddRange(await item.Common.GetRecommendedPlaylistAsync(null, 6));
        // }
        // ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        // {
        //     foreach (var item in recommendedPlaylist)
        //     {
        //         RecommendedPlaylist.Add(item);
        //     }
        // });
    }

}