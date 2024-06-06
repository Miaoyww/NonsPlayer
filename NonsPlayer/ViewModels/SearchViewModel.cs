using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Security.Cryptography;
using System.Text;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class SearchViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    [ObservableProperty] private Artist[]? artists;
    public ObservableCollection<MusicModel> MusicItems = new();
    [ObservableProperty] private Playlist[]? playlists;
    private string queryKey;

    public void OnNavigatedTo(object parameter)
    {
        queryKey = (parameter as string).ToLower();
        Search(queryKey).ConfigureAwait(false);
    }

    public void OnNavigatedFrom()
    {
    }

    public void DoubleClick(object sender, DoubleTappedRoutedEventArgs e)
    {
        var listView = sender as ListView;
        if (listView.SelectedItem is MusicModel item) PlayQueue.Instance.Play(item.Music);
    }

    public async Task Search(string key)
    {
        // var searcher = await CacheHelper.GetSearchResultAsync(GetB64(key), key);
        // await Task.WhenAll(
        //     searcher.SearchMusics(),
        //     searcher.SearchArtists(),
        //     searcher.SearchPlaylists());
        // SearchHelper.Instance.BestMusicResult = searcher.Musics[0];
        // Artists = searcher.Artists;
        // Playlists = searcher.Playlists;
        // for (var i = 0; i < searcher.Musics.Count(); i++)
        // {
        //     var index = i;
        //     if (index < AppConfig.PlaylistTrackShowCount)
        //         ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        //         {
        //             MusicItems.Add(new MusicModel
        //             {
        //                 Music = searcher.Musics[index],
        //                 Index = (index + 1).ToString("D2")
        //             });
        //         });
        // }
    }

    public async void OnScrollViewerViewChanged(object? sender, ScrollViewerViewChangedEventArgs e)
    {
        //
        // if (sender is ScrollViewer scrollViewer)
        // {
        //     var offset = scrollViewer.VerticalOffset;
        //
        //     var height = scrollViewer.ScrollableHeight;
        //     if (height - offset <
        //         App.GetService<ILocalSettingsService>().GetOptions().PlaylistTrackShowCount &&
        //         currentItemGroupIndex < playListObject.MusicsCount - 1)
        //         await LoadMusicItemsByGroup();
        // }
    }

    private string GetB64(string kyw)
    {
        return Convert.ToBase64String(MD5.HashData(Encoding.UTF8.GetBytes(kyw)));
    }
}