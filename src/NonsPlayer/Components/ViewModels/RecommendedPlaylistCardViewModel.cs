using System.Collections.ObjectModel;
using Windows.UI;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Contracts.Adapters;
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
    [ObservableProperty] private Brush fontColor;
    [ObservableProperty] private Visibility tipVisibility = Visibility.Collapsed;
    private IMusic[] songs;
    public async void Init(IMusic[] music)
    {
        if (music != null)
        {
            var firstMusic = music[0];
            Cover = (await CacheHelper.GetImageBrushAsync(firstMusic.CacheAvatarId,
                firstMusic.GetCoverUrl())).ImageSource;
            TipVisibility = Visibility.Collapsed;
            FontColor = App.Current.Resources["LightTextColor"] as SolidColorBrush;
            songs = music;
            return;
        }

        TipVisibility = Visibility.Visible;
        FontColor = App.Current.Resources["CommonTextColor"] as SolidColorBrush;
    }

    public void RecommendedPlaylistCard_OnTapped(object sender, TappedRoutedEventArgs e)
    {
        if (songs == null) return;
        var playlist = CacheHelper.GetPlaylist("DailyRecommendedPlaylist", () =>
        {
            var playlist = new RecommendedPlaylist
            {
                Name = "DailyRecommendedPlaylist".GetLocalized(),
                Id = "DailyRecommendedPlaylist",
                Musics = new List<IMusic>(songs),
                Creator = string.Empty,
                CreateTime = DateTime.Today,
                Description = string.Empty,
                MusicTrackIds = songs.Select(m => m.Id).ToArray(),
                AvatarUrl = songs[0].AvatarUrl,
                IsInitialized = true
            };
            return playlist;
        });
 
        ServiceHelper.NavigationService.NavigateTo(typeof(PlaylistDetailViewModel)?.FullName, playlist);
    }
    
    
    public class RecommendedPlaylist: IPlaylist
    {
        public string Id { get; set; }
        public string Md5 { get; set; }
        public string Name { get; set; }
        public string ShareUrl { get; set; }
        public string AvatarUrl { get; set; }
        public DateTime CreateTime { get; set; }
        public string Creator { get; set; }
        public string Description { get; set; }
        public string[] MusicTrackIds { get; set; }
        public string[] Tags { get; set; }
        public IAdapter Adapter { get; set; }
        public List<IMusic> Musics { get; set; }
        public bool IsInitialized { get; set; }
        public Task InitializePlaylist()
        {
            return Task.CompletedTask;
        }

        public Task InitializeMusics()
        {
            return Task.CompletedTask;
        }

        public Task InitializeTracks()
        {
            return Task.CompletedTask;
        }
    }
}