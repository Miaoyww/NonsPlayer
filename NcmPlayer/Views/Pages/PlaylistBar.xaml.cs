using NcmPlayer.CloudMusic;
using NcmPlayer.Player;
using NcmPlayer.Resources;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Linq;
using System.Text;
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
using static NcmPlayer.Resources.Resources;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// PlayListBar.xaml 的交互逻辑
    /// </summary>
    public partial class PlaylistBar : Page, INotifyPropertyChanged
    {
        public PlaylistBar()
        {
            InitializeComponent();
            DataContext = Res.res;
            label_songsCount.DataContext = Res.res;
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        public void UpdateSongsList(List<Song> songs)
        {
            int gridCount = songs.Count;
            for (int index = 0; index < gridCount; index++)
            {
                Song one = songs[index];
                Res.wholePlaylist.Add(Res.wholePlaylist.Song2Vis(one));
                UpdateInfo();
            }
        }

        public void Play(Song song)
        {
            UpdateSongsList(new List<Song>() { song });
            Res.wholePlaylist.Play(Res.wholePlaylist.Song2Vis(song));
        }

        public void Play(string id)
        {
            Res.wholePlaylist.Play(id);
            UpdateInfo();
        }

        public void ClearSongs()
        {
            Playlist.Items.Clear();
            Res.wholePlaylist.Clear();
        }

        private void UpdateInfo()
        {
            Playlist.Items.Clear();
            for (int index = 0;index < Res.wholePlaylist.Count; index ++)
            {
                Playlist.Items.Add(Res.wholePlaylist.GetSongVis(index).View);
            }
        }

        private void Playlist_PreviewMouseDoubleClick(object sender, MouseButtonEventArgs e)
        {
            Res.wholePlaylist.Play(Playlist.SelectedIndex);
        }

    }
}
