using System.ComponentModel;
using System.Runtime.CompilerServices;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels
{
    [INotifyPropertyChanged]
    public partial class UserPlaylistCardViewModel
    {
        [ObservableProperty]
        private string? name = string.Empty;
        
        [ObservableProperty]
        private ImageBrush? cover;
        
        public string Uid
        {
            get;
            set;
        }

        public void Init(JObject playlistItem)
        {
            Name = playlistItem["name"].ToString();
            Cover = new ImageBrush
            {
                ImageSource = new BitmapImage(new Uri(playlistItem["coverImgUrl"].ToString() + "?param=40y40"))
            };
            Uid = playlistItem["id"].ToString();
        }
        public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e) =>
            PlaylistHelper.OpenMusicListDetail(long.Parse(Uid), ServiceHelper.NavigationService);

    }
}