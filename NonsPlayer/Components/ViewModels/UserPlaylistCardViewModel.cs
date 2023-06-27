using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Framework.Resources;
using NonsPlayer.Helpers;

namespace NonsPlayer.Components.ViewModels
{
    public class UserPlaylistCardViewModel : INotifyPropertyChanged
    {
        private string _name = string.Empty;
        private ImageBrush? _cover;

        public string Name
        {
            get => _name;
            set
            {
                _name = value;
                OnPropertyChanged();
            }
        }

        public ImageBrush? Cover
        {
            get => _cover;
            set
            {
                _cover = value;
                OnPropertyChanged();
            }
        }

        public string Uid
        {
            get;
            set;
        }

        public async void Init(JObject playlistItem)
        {
            Name = playlistItem["name"].ToString();
            Cover = new ImageBrush
            {
                ImageSource = new BitmapImage(new Uri(playlistItem["coverImgUrl"].ToString() + "?param=40y40"))
            };
            Uid = playlistItem["id"].ToString();
        }
        public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e) =>
            Tools.OpenMusicListDetail(long.Parse(Uid), ServiceEntry.NavigationService);
        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

    }
}