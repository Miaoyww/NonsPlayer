using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.ViewModels;

public partial class RecommendedPlaylistCardViewModel : ObservableObject
{
    [ObservableProperty] private ImageSource cover;

    public RecommendedPlaylistCardViewModel()
    {

    }

    public async void Init(IPlaylist playList)
    {
        if (!playList.IsInitialized) await playList.InitializeMusics();
        var firstMusic = playList.Musics[0];
        Cover = (await CacheHelper.GetImageBrushAsync(firstMusic.CacheAvatarId,
            firstMusic.GetCoverUrl())).ImageSource;
    }
}