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
using NeteaseCloudMusicControl.Views.Methods;
using System.Timers;
using NeteaseCloudMusicControl.Service;

namespace NeteaseCloudMusicControl.Views.Pages
{
    public partial class Home : Page
    {
        public Home()
        {
            InitializeComponent();
        }

        private void Button_Click(object sender, RoutedEventArgs e)
        {
            Song song = new(tbox_id.Text);
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
            BitmapImage image = new BitmapImage();
            image.BeginInit();
            image.StreamSource = song.Cover;
            image.EndInit();
            ImageBrush brush = new();
            brush.ImageSource = image;
            MainWindow.mainWindow.btn_albumPic.Background = brush;
            MainWindow.mainWindow.tblock_title.Text = name;
            MainWindow.mainWindow.tblock_artists.Text = artists;
            Player.playerPage.b_image.Background = brush;
            Player.playerPage.tblock_title.Text = name;
            Player.playerPage.tblock_artists.Text = artists;
            PlayerMethods.RePlay(path);
        }
    }
}
