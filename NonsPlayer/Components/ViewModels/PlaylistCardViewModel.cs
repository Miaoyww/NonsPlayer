using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class PlaylistCardViewModel
{
    [ObservableProperty] private ImageBrush cover;
    [ObservableProperty] private string title;
    [ObservableProperty] private string id;

    public void Init(JObject item)
    {
        var picUrl = item["picUrl"] + "?param=200y200";
        var title = (string)item["name"];
        Id = (string)item["id"];
        Title = title;
        Cover = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(picUrl))
        };
    }

    public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e) =>
        PlaylistHelper.OpenMusicListDetail(long.Parse(id), ServiceHelper.NavigationService);
}