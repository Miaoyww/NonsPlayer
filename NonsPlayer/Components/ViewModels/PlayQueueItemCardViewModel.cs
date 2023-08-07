using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class PlayQueueItemCardViewModel
{
    [ObservableProperty] private string artists;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private bool liked; //TODO: Implement this
    [ObservableProperty] private Music music;
    [ObservableProperty] private string name;
    [ObservableProperty] private string time;

    partial void OnMusicChanged(Music music)
    {
        Name = music.Name;
        Artists = music.ArtistsName;
        Time = music.TotalTimeString;
        Cover = CacheHelper.GetImageBrush(music.Album.CacheSmallAvatarId, music.Album.SmallAvatarUrl);
    }
}