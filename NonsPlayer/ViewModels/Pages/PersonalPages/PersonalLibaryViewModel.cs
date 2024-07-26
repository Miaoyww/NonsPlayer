using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class PersonalLibaryViewModel : ObservableRecipient, INavigationAware
{
    [ObservableProperty] private IPlaylist favoriteSongs;
    [ObservableProperty] private int favoriteCount;
    [ObservableProperty] private string userName;
    [ObservableProperty] private ImageBrush avatar;
    private IAccount account;
    private IAdapter adapter;

    public async void OnNavigatedTo(object parameter)
    {
        adapter = parameter as IAdapter;
        await Task.Run(Init);
    }

    public void OnNavigatedFrom()
    {
    }

    private async Task Init()
    {
        account = adapter.Account.GetAccount();
        var avatar = await account.GetAvatarUrlAsync();
        if (account.IsLoggedIn)
        {
            if (adapter.Account.FavoritePlaylist == null)
            {
                await adapter.Account.GetFavoritePlaylist();
            }

            ServiceHelper.DispatcherQueue.TryEnqueue(() =>
            {
                Avatar = new ImageBrush{ImageSource = new BitmapImage(new Uri(avatar))};
                if (adapter.Account.FavoritePlaylist != null)
                {
                    FavoriteSongs = adapter.Account.FavoritePlaylist;
                    FavoriteCount = FavoriteSongs.MusicsCount;
                    UserName = account.Name;
                }
            });
        }
    }

    [RelayCommand]
    public async Task PlayFavorite()
    {
        PlayQueue.Instance.Clear();
        if (FavoriteSongs.Musics == null)
        {
            await FavoriteSongs.InitializeTracks();
            await FavoriteSongs.InitializeMusics();
        }
        PlayQueue.Instance.AddMusicList(FavoriteSongs.Musics.ToArray());
        PlayQueue.Instance.PlayNext();
    }
}