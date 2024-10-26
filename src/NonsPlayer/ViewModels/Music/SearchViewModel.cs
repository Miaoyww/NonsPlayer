﻿using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Security.Cryptography;
using System.Text;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Cache;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public partial class SearchViewModel : ObservableRecipient, INavigationAware, INotifyPropertyChanged
{
    public ObservableCollection<MusicModel> MusicModels = new();
    public ObservableCollection<IMusic> Songs = new();
    public ObservableCollection<IArtist> Artists = new();
    [ObservableProperty] private List<IPlaylist> playlists = new();
    [ObservableProperty] private IMusic firstMusic;
    [ObservableProperty] private IArtist firstArtist;

    private CacheService cacheService = App.GetService<CacheService>();

    public async void OnNavigatedTo(object parameter)
    {
        await Search((string)parameter);
    }

    public void OnNavigatedFrom()
    {
    }


    private async Task Search(string keywords)
    {
        var cacheId = $"search_result_{keywords}";
        var cache = CacheHelper.GetSearchResult(cacheId);
        List<SearchResult> resuts = new();
        if (cache != null)
        {
            resuts = cache;
        }
        else
        {
            var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Search);
            foreach (IAdapter adapter in adapters)
            {
                var result = await adapter.Search.SearchAsync(keywords);
                resuts.Add(result);
            }

            cacheService.AddOrUpdate(cacheId, resuts);
        }

        foreach (SearchResult searchResult in resuts)
        {
            foreach (IMusic resultMusic in searchResult.Musics)
            {
                Songs.Add(resultMusic);
            }
        }

        foreach (SearchResult result in resuts)
        {
            foreach (var artist in result.Artists)
            {
                Artists.Add(artist);
            }
        }

        foreach (SearchResult searchResult in resuts)
        {
            foreach (IPlaylist searchResultPlaylist in searchResult.Playlists)
            {
                Playlists.Add(searchResultPlaylist);
            }
        }

        if (Songs.Count != 0)
        {
            FirstMusic = Songs[0];
        }

        if (Artists.Count != 0)
        {
            FirstArtist = Artists[0];
        }

        for (var i = 0; i < Songs.Count; i++)
        {
            MusicModels.Add(new MusicModel { Music = Songs[i], Index = i.ToString("D2") });
        }
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
        //         App.GetService<ILocalSettingsService>().GetOptions().PlaylistTrackCount &&
        //         currentItemGroupIndex < playListObject.MusicsCount - 1)
        //         await LoadMusicItemsByGroup();
        // }
    }

    private string GetB64(string kyw)
    {
        return Convert.ToBase64String(MD5.HashData(Encoding.UTF8.GetBytes(kyw)));
    }
}