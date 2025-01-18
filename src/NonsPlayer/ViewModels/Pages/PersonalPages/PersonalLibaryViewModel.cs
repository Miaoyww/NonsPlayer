using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Utils;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using System.Collections.ObjectModel;

namespace NonsPlayer.ViewModels;

public partial class PersonalLibaryViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private IPlaylist? favoriteSongs;
    [ObservableProperty] private int favoriteCount;
    [ObservableProperty] private string userName;
    [ObservableProperty] private ImageBrush avatar;
    [ObservableProperty] private string selected;
    [ObservableProperty] private string lyric;

    private ILogger logger = App.GetLogger<PersonalLibaryViewModel>();
    public ObservableCollection<PlaylistModel> Playlists = new();
    private PlaylistModel[] SavedPlaylists;
    private PlaylistModel[] CreatedPlaylists;

    private IAccount account;
    public IAdapter Adapter;

    public void OnNavigatedTo(object parameter)
    {
        Adapter = parameter as IAdapter;
        Task.Run(Init);
    }

    public void OnNavigatedFrom()
    {
    }

    private async Task Init()
    {
        account = Adapter.Account.GetAccount();
        if (account.IsLoggedIn)
        {
            Task.Run(async () =>
            {
                var avatar = await account.GetAvatarUrlAsync();
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    Avatar = new ImageBrush { ImageSource = new BitmapImage(new Uri(avatar)) };
                    UserName = account.Name;
                });
            });

            Task.Run(async () =>
            {
                FavoriteSongs = await CacheHelper.GetPlaylistAsync($"playlist_favorite_{Adapter.GetMetadata().Slug}",
                    async () =>
                    {
                        await Adapter.Account.GetFavoritePlaylist();
                        return Adapter.Account.FavoritePlaylist;
                    });
                var songs = Adapter.Account.FavoritePlaylist.Musics.Take(20)
                    .Select(music => new MusicModel { Music = music }).ToArray();

                Task.Run(async () =>
                {
                    var firstSong = songs.GetValue(Random.Shared.Next(0, 19));
                    await LoadLyric((firstSong as MusicModel)?.Music);
                    await SwitchPlaylistAsync("all");
                });

                if (Adapter.Account.FavoritePlaylist == null) return;
                Task.WhenAny(FavoriteSongs.InitializeTracks(), FavoriteSongs.InitializeMusics());
                ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                {
                    if (Adapter.Account.FavoritePlaylist != null)
                    {
                        FavoriteCount = FavoriteSongs.MusicsCount;
                    }
                });
            });
        }
    }

    private async Task LoadLyric(IMusic? firstSong)
    {
        try
        {
            if (firstSong != null)
            {
                var pureLyric = await firstSong.GetLyric();
                if (pureLyric != null)
                {
                    ServiceHelper.DispatcherQueue.TryEnqueue(() =>
                    {
                        Lyric = string.Join("\n", pureLyric.LyricLines.Take(3).Select(x => x.Pure).ToArray());
                    });
                }
            }
        }
        catch (Exception e)
        {
            logger.LogError(e, "Failed to get lyric");
        }
    }

    [RelayCommand]
    public void PlayFavorite()
    {
        if (FavoriteSongs == null) return;
        if (FavoriteSongs.MusicsCount == 0) return;
        PlayQueue.Instance.Clear();
        PlayQueue.Instance.AddMusicList(FavoriteSongs.Musics.ToArray());
        PlayQueue.Instance.PlayNext();
    }

    [RelayCommand]
    public async Task SwitchPlaylistAsync(string param)
    {
        if (CreatedPlaylists == null) await LoadUserPlaylists();
        if (SavedPlaylists == null) await LoadUserPlaylists();

        if (param == "saved")
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Playlists.Clear();
                foreach (PlaylistModel playlist in SavedPlaylists)
                {
                    Playlists.Add(playlist);
                }

                Selected = "Saved Playlists";
            });
        }
        else if (param == "created")
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Playlists.Clear();
                foreach (PlaylistModel playlist in CreatedPlaylists)
                {
                    Playlists.Add(playlist);
                }

                Selected = "Created Playlists";
            });
        }
        else if (param == "all")
        {
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Playlists.Clear();
                foreach (PlaylistModel playlist in CreatedPlaylists)
                {
                    Playlists.Add(playlist);
                }

                foreach (PlaylistModel playlist in SavedPlaylists)
                {
                    Playlists.Add(playlist);
                }

                Selected = "All Playlists";
            });
        }
    }

    private async Task LoadUserPlaylists()
    {
        if (Adapter.Account.GetAccount().IsLoggedIn)
        {
            if (Adapter.Account.SavedPlaylists == null) await Adapter.Account.GetUserPlaylists();
            if (Adapter.Account.CreatedPlaylists == null) await Adapter.Account.GetUserPlaylists();

            SavedPlaylists =
                Adapter.Account.SavedPlaylists.Select(x => new PlaylistModel { Playlist = x }).ToArray();
            CreatedPlaylists =
                Adapter.Account.CreatedPlaylists.Select(x => new PlaylistModel { Playlist = x }).ToArray();
        }
    }
}