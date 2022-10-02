using NcmPlayer.Framework.Model;
using NcmPlayer.Resources;
using System.Collections.Generic;
using System.ComponentModel;
using System.Windows.Controls;
using System.Windows.Input;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// PlayListBar.xaml 的交互逻辑
    /// </summary>
    public partial class PlayListBar : Page, INotifyPropertyChanged
    {
        public PlayListBar()
        {
            InitializeComponent();
            DataContext = ResEntry.res;
            label_musicsCount.DataContext = ResEntry.res;
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        public void UpdateMusicsList(List<Music> musics)
        {
            int gridCount = musics.Count;
            for (int index = 0; index < gridCount; index++)
            {
                Music one = musics[index];
                ResEntry.wholePlaylist.Add(ResEntry.wholePlaylist.Music2Vis(one));
                UpdateInfo();
            }
        }

        public void Play(Music music)
        {
            UpdateMusicsList(new List<Music>() { music });
            ResEntry.wholePlaylist.Play(ResEntry.wholePlaylist.Music2Vis(music));
        }

        public void Play(long id)
        {
            ResEntry.wholePlaylist.Play(id);
            UpdateInfo();
        }

        public void ClearMusics()
        {
            Playlist.Items.Clear();
            ResEntry.wholePlaylist.Clear();
        }

        private void UpdateInfo()
        {
            Playlist.Items.Clear();
            for (int index = 0; index < ResEntry.wholePlaylist.Count; index++)
            {
                Playlist.Items.Add(ResEntry.wholePlaylist.GetMusicVis(index).View);
            }
        }

        private void Playlist_PreviewMouseDoubleClick(object sender, MouseButtonEventArgs e)
        {
            if (Playlist.SelectedIndex != -1)
            {
                ResEntry.wholePlaylist.Play(Playlist.SelectedIndex);
            }
        }
    }
}