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
using NcmPlayer.Player;
using NcmPlayer.CloudMusic;

namespace NcmPlayer.Views.Pages
{
    public partial class Home : Page
    {
        public Home()
        {
            InitializeComponent();
            DataContext = Res.res;
        }

        private void Button_Click(object sender, RoutedEventArgs e)
        {
            Thread thread = new(() =>
            {
                Dispatcher.BeginInvoke(new Action(() =>
                {
                    CloudMusic.PlayList playList = new(tbox_id.Text);
                    string name = playList.Name;
                    string creator = playList.Creator;
                    Stream playlistCover = playList.Cover;
                    string description = playList.Description;
                    string createTime = playList.CreateTime.ToString();
                    int songsCount = playList.SongsCount;
                    Playlist newone = new();
                    MainWindow.mainWindow.PageFrame.Content = newone;
                    newone.Name = name;
                    newone.Creator = creator;
                    newone.CreateTime = createTime;
                    newone.SetCover(playlistCover);
                    newone.Description = description;
                    newone.SongsCount = songsCount.ToString();
                    Song[] songs = playList.InitArtWorkList(0, 20);
                    newone.UpdateSongsList(songs);
                }));
            });
            thread.IsBackground = true;
            thread.Start();
        }
    }
}
