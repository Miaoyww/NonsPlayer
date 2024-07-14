using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Services;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class RecentlyPlayItemCardViewModel
{
    public IMusic Music;
    [ObservableProperty] public ImageBrush cover;

    public void Init(IMusic music)
    {
        Music = music;
        cover = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(music.AvatarUrl))
        };
    }

    public void Init(string url)
    {
        cover = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(url))
        };
    }
}