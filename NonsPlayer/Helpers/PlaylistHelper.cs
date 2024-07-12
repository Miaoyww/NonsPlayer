using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Helpers;

public static class PlaylistHelper
{

    public static void OpenMusicListDetail(IPlaylist playlist, INavigationService navigationService)
    {
        navigationService.NavigateTo(typeof(PlaylistDetailViewModel).FullName!, playlist);
    }
}