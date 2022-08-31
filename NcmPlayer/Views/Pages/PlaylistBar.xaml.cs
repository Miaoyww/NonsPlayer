﻿using NcmPlayer.CloudMusic;
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
            DataContext = ResEntry.res;
            label_songsCount.DataContext = ResEntry.res;
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        public void UpdateSongsList(List<Song> songs)
        {
            int gridCount = songs.Count;
            for (int index = 0; index < gridCount; index++)
            {
                Song one = songs[index];
                ResEntry.wholePlaylist.Add(ResEntry.wholePlaylist.Song2Vis(one));
                UpdateInfo();
            }
        }

        public void Play(Song song)
        {
            UpdateSongsList(new List<Song>() { song });
            ResEntry.wholePlaylist.Play(ResEntry.wholePlaylist.Song2Vis(song));
        }

        public void Play(string id)
        {
            ResEntry.wholePlaylist.Play(id);
            UpdateInfo();
        }

        public void ClearSongs()
        {
            Playlist.Items.Clear();
            ResEntry.wholePlaylist.Clear();
        }

        private void UpdateInfo()
        {
            Playlist.Items.Clear();
            for (int index = 0;index < ResEntry.wholePlaylist.Count; index ++)
            {
                Playlist.Items.Add(ResEntry.wholePlaylist.GetSongVis(index).View);
            }
        }

        private void Playlist_PreviewMouseDoubleClick(object sender, MouseButtonEventArgs e)
        {
            ResEntry.wholePlaylist.Play(Playlist.SelectedIndex);
        }

    }
}