using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

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
}