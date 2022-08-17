using Microsoft.Win32;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;
using System.Timers;
using NcmPlayer.Service;
using NcmPlayer;
using NeteaseCloudMusicControl;

namespace NcmPlayer.Views.Pages
{
    public partial class Home : Page
    {
        public Home()
        {
            InitializeComponent();
        }

        private void Button_Click(object sender, RoutedEventArgs e)
        {
            Song song;
            try
            {
                song = new(tbox_id.Text);
            }
            catch (InvalidCastException er)
            {
                MainWindow.ShowMyDialog(er.Message, "错误");
                return;
            }
            
            string path = song.GetFile();
            string[] artist = song.Artists;
            string name = song.Name;
            string artists = string.Empty;
            for(int i = 0;i <= artist.Length - 1; i++)
            {
                if (i != artist.Length - 1)
                {
                    artists += artist[i] + "/";
                }
                else
                {
                    artists += artist[i];
                }
            }
            MusicPlayer.RePlay(path, name, artists, song.Cover);
            Res.res.CPlayAlbumPicUrl = song.coverUrl;
            Res.res.CPlayAlbumId = song.AlbumId;
        }
    }
}
