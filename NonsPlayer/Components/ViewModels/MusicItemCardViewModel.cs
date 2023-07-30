using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Cache;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Services;

namespace NonsPlayer.Components.ViewModels;

public partial class MusicItemCardViewModel : ObservableObject
{
    [ObservableProperty] private string name;
    [ObservableProperty] private string artists;
    [ObservableProperty] private string time;
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string album;
    [ObservableProperty] private bool liked;
    [ObservableProperty] private string index;
    public Music Music;

    public async void Init(Music music)
    {
        Music = music;
        await Task.Run(() =>
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Cover = CacheHelper.GetImageBrush(music.Album.CacheSmallCoverId, music.Album.SmallCoverUrl);
                Name = Music.Name;
                Time = Music.TotalTimeString;
                Album = Music.AlbumName;
                Artists = string.IsNullOrEmpty(Music.ArtistsName) ? "未知艺人" : Music.ArtistsName;
                Liked = UserPlaylistHelper.Instance.IsLiked(Music.Id);
            });
        });
    }


    public void Play(object sender, PointerRoutedEventArgs e)
    {
        PlayQueue.Instance.Play(Music);
    }
}