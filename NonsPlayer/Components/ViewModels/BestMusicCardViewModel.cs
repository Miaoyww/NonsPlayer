using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class BestMusicCardViewModel
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private long? id;
    [ObservableProperty] private string subtitle;
    [ObservableProperty] private string title;

    public BestMusicCardViewModel()
    {
        SearchHelper.Instance.BestMusicResultChanged += OnBestMusicResultChanged;
    }

    public async void OnBestMusicResultChanged(Music value)
    {
        Id = value.Id;
        Title = value.Name;
        Subtitle = value.ArtistsName;
        Cover = await CacheHelper.GetImageBrushAsync(value.Album.CacheMiddleAvatarId, value.Album.MiddleAvatarUrl);
    }

    [RelayCommand]
    public void Play()
    {
        PlayQueue.Instance.Play(SearchHelper.Instance.BestMusicResult);
    }
}