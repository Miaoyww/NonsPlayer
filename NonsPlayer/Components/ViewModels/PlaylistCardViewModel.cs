using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Cache;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class PlaylistCardViewModel
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string title;
    [ObservableProperty] private string id;

    public async void Init(Playlist item)
    {
        await item.LoadAsync();
        Id = item.Id.ToString();
        Title = item.Name;
        Cover = CacheHelper.GetImageBrush(item.CacheMiddleCoverId, item.MiddleCoverUrl);
    }

    public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e) =>
        PlaylistHelper.OpenMusicListDetail(long.Parse(id), ServiceHelper.NavigationService);
}