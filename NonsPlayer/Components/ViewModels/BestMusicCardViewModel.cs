using System.ComponentModel;
using System.Runtime.CompilerServices;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Models;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class BestMusicCardViewModel
{
    [ObservableProperty] private long? id;
    [ObservableProperty] private string title;
    [ObservableProperty] private string subtitle;
    [ObservableProperty] private ImageBrush cover;

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
}