using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class RecommendedPlaylistCardViewModel
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string id;
    [ObservableProperty] private string title;

    public void Init(Playlist item)
    {
        Id = item.Id.ToString();
        Title = item.Name;
        Cover = CacheHelper.GetImageBrush(item.CacheMiddleAvatarId, item.MiddleAvatarUrl);
    }

    public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e)
    {
        PlaylistHelper.OpenMusicListDetail(long.Parse(id), ServiceHelper.NavigationService);
    }
}