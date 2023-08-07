using System.ComponentModel;
using System.Runtime.CompilerServices;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Models;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class BestArtistCardViewModel
{
    [ObservableProperty] private ImageSource cover;
    [ObservableProperty] private long? id;
    [ObservableProperty] private string name;

    public BestArtistCardViewModel()
    {
        SearchHelper.Instance.BestMusicResultChanged += OnBestMusicResultChanged;
    }

    private async void OnBestMusicResultChanged(Music value)
    {
        Id = value.Artists[0].Id;
        Name = value.Artists[0].Name;
        Cover = (await CacheHelper.GetImageBrushAsync(value.Artists[0].CacheAvatarId, value.Artists[0].AvatarUrl)).ImageSource;
    }
}