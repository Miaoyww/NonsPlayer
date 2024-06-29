using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Cache;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class UserPlaylistCardViewModel
{
    [ObservableProperty] private ImageBrush? cover;
    [ObservableProperty] private string? name = string.Empty;
    public Playlist PurePlaylist;
    public string Uid;

    public void Init(Playlist playlistItem)
    {
        CacheManager.Instance.Set(playlistItem.CacheId, new CacheItem<Playlist> { Data = playlistItem });
        PurePlaylist = playlistItem;
        Name = playlistItem.Name;
        Uid = playlistItem.Id.ToString();
        Cover = CacheHelper.GetImageBrush(playlistItem.CacheSmallAvatarId, playlistItem.SmallAvatarUrl);
    }

    public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e)
    {
        PlaylistHelper.OpenMusicListDetail(PurePlaylist, ServiceHelper.NavigationService);
    }
}