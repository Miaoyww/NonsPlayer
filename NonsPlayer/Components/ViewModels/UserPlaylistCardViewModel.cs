using System.ComponentModel;
using System.Runtime.CompilerServices;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Cache;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels
{
    [INotifyPropertyChanged]
    public partial class UserPlaylistCardViewModel
    {
        [ObservableProperty] private string? name = string.Empty;

        [ObservableProperty] private ImageBrush? cover;

        public string Uid;

        public void Init(Playlist playlistItem)
        {
            CacheManager.Instance.Set(playlistItem.CacheId, new CacheItem<Playlist> {Data = playlistItem});
            Name = playlistItem.Name;
            Uid = playlistItem.Id.ToString();
            Cover = CacheHelper.GetImageBrush(playlistItem.CacheSmallCoverId, playlistItem.SmallCoverUrl);
        }

        public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e) =>
            PlaylistHelper.OpenMusicListDetail(long.Parse(Uid), ServiceHelper.NavigationService);
    }
}