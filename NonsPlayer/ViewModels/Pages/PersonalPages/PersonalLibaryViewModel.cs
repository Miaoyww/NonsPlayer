using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using System.Collections.ObjectModel;

namespace NonsPlayer.ViewModels;

public partial class PersonalLibaryViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private IPlaylist favoriteSongs;
    [ObservableProperty] private int favoriteCount;
    [ObservableProperty] private string userName;
    [ObservableProperty] private ImageBrush avatar;
    public ObservableCollection<MusicModel> TopSongs = new();
    private IAccount account;
    public IAdapter Adapter;

    public async void OnNavigatedTo(object parameter)
    {
        Adapter = parameter as IAdapter;
        await Task.Run(Init);
    }

    public void OnNavigatedFrom()
    {
    }

    private async Task Init()
    {
        account = Adapter.Account.GetAccount();
        if (account.IsLoggedIn)
        {
            var avatar = await account.GetAvatarUrlAsync();
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Avatar = new ImageBrush { ImageSource = new BitmapImage(new Uri(avatar)) };
                UserName = account.Name;
            });

            await Adapter.Account.GetFavoritePlaylist();
            FavoriteSongs = Adapter.Account.FavoritePlaylist;

            if (Adapter.Account.FavoritePlaylist == null) return;

            Task.WaitAll(FavoriteSongs.InitializeTracks(), FavoriteSongs.InitializeMusics());
            var songs = Adapter.Account.FavoritePlaylist.Musics.Take(20)
                .Select(music => new MusicModel { Music = music });
            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                foreach (MusicModel item in songs)
                {
                    TopSongs.Add(item);
                }
                if (Adapter.Account.FavoritePlaylist != null)
                {
                    FavoriteCount = FavoriteSongs.MusicsCount;
                }
            });
        }
    }

    [RelayCommand]
    public void PlayFavorite()
    {
        PlayQueue.Instance.Clear();
        PlayQueue.Instance.AddMusicList(FavoriteSongs.Musics.ToArray());
        PlayQueue.Instance.PlayNext();
    }
}