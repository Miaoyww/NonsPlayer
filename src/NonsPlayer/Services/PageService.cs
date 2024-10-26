﻿using CommunityToolkit.Mvvm.ComponentModel;

using Microsoft.UI.Xaml.Controls;

using NonsPlayer.Contracts.Services;
using NonsPlayer.ViewModels;
using NonsPlayer.Views;
using NonsPlayer.Views.Local;
using NonsPlayer.Views.Pages;

namespace NonsPlayer.Services;

public class PageService : IPageService
{
    private readonly Dictionary<string, Type> _pages = new();

    public PageService()
    {
        Configure<SettingsViewModel, SettingsPage>();
        Configure<HomeViewModel, HomePage>();
        Configure<ExploreViewModel, ExplorePage>();
        Configure<PlaylistDetailViewModel, PlaylistDetailPage>();
        Configure<PersonalCenterViewModel, PersonalCenterPage>();
        Configure<SearchViewModel, SearchPage>();
        Configure<ArtistViewModel, ArtistPage>();
        Configure<LyricViewModel, LyricPage>();
        Configure<AlbumViewModel, AlbumPage>();
        Configure<UpdateViewModel, UpdatePage>();
        Configure<LocalViewModel, LocalPage>();
        Configure<TestViewModel, TestPage>();
        Configure<AdapterManagerViewModel, AdapterManagerPage>();
        Configure<PersonalLibaryViewModel, PersonalLibaryPage>();
        Configure<LoginViewModel, LoginPage>();
        Configure<LocalMusicLibViewModel, LocalMusicLibPage>();
        Configure<LocalQueueViewModel, LocalQueuePage>();
    }

    public Type GetPageType(string key)
    {
        Type? pageType;
        lock (_pages)
        {
            if (!_pages.TryGetValue(key, out pageType))
                throw new ArgumentException($"Page not found: {key}. Did you forget to call PageService.Configure?");
        }

        return pageType;
    }

    private void Configure<VM, V>()
        where VM : ObservableObject
        where V : Page
    {
        lock (_pages)
        {
            var key = typeof(VM).FullName!;
            if (_pages.ContainsKey(key))
                throw new ArgumentException($"The key {key} is already configured in PageService");

            var type = typeof(V);
            if (_pages.Any(p => p.Value == type))
                throw new ArgumentException(
                    $"This type is already configured with key {_pages.First(p => p.Value == type).Key}");

            _pages.Add(key, type);
        }
    }
}
