using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using Windows.ApplicationModel.DataTransfer;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class RecentlyPlayItemCardViewModel
{
    public IMusic Music;
    [ObservableProperty] public ImageBrush cover;

    public void Init(IMusic music)
    {
        Music = music;
        Cover = CacheHelper.GetImageBrush(music.Album.CacheAvatarId, music.GetCoverUrl("?param=200x200"));
    }

    [RelayCommand]
    public void PlayNext()
    {
        PlayQueue.Instance.AddNext(Music);
    }

    [RelayCommand]
    public void CheckAlbum()
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(AlbumViewModel)?.FullName, Music.Album);
    }

    [RelayCommand]
    public void CopyMusicInfo()
    {
        var data = new DataPackage();
        data.SetText($"Share music: {Music.Name} made by {Music.ArtistsName}");
        Clipboard.SetContent(data);
    }

    [RelayCommand]
    public void CopyShareUrl()
    {
        var data = new DataPackage();
        data.SetText(Music.ShareUrl);
        Clipboard.SetContent(data);
    }
}