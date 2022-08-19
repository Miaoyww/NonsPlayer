using NcmPlayer.CloudMusic;
using NcmPlayer.Player;
using NcmPlayer.Views.Methods;
using System;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// Album.xaml 的交互逻辑
    /// </summary>
    public partial class Playlist : Page
    {
        public Playlist()
        {
            InitializeComponent();
            DataContext = new ViewModel();
        }

        public Grid[] grids;

        public string Name
        {
            set => PlaylistName.Text = value;
        }

        public string CreateTime
        {
            set => PlaylistCreateTime.Text = value;
        }

        public string SongsCount
        {
            set => PlaylistSongsCount.Text = $"{value} 首歌曲";
        }

        public string Description
        {
            set => PlaylistDescription.Text = value;
        }

        public string Creator
        {
            set => PlaylistCreator.Text = value;
        }

        public void SetCover(Stream stream)
        {
            BitmapImage image = new();
            image.BeginInit();
            image.StreamSource = stream;
            image.EndInit();
            ImageBrush brush = new();
            brush.ImageSource = image;
            PlaylistCover.Background = brush;
        }

        public void UpdateSongsList(Song[] songs)
        {
            int gridCount = songs.Length - 1;
            grids = new Grid[songs.Length];
            for (int index = 0; index < gridCount; index++)
            {
                Song one = songs[index];
                grids[index] = new Grid();
                grids[index].Tag = one.Id;
                string artists = string.Empty;
                for (int i = 0; i <= one.Artists.Length - 1; i++)
                {
                    if (i != one.Artists.Length - 1)
                    {
                        artists += one.Artists[i] + "/";
                    }
                    else
                    {
                        artists += one.Artists[i];
                    }
                }
                Label label_index = new()
                {
                    Content = (index + 1).ToString(),
                    FontSize = 20,
                    Padding = new Thickness(0, 5, 0, 0),
                    HorizontalAlignment = HorizontalAlignment.Left,
                    VerticalAlignment = VerticalAlignment.Center
                };
                Separator separator = new()
                {
                    BorderThickness = new Thickness(0)
                };
                TextBlock tblock_Name = new()
                {
                    Text = one.Name,
                    HorizontalAlignment = HorizontalAlignment.Left,
                    VerticalAlignment = VerticalAlignment.Center,
                    FontSize = 20,
                    Margin = new Thickness(50, 0, 0, 0)
                };
                TextBlock tblock_Artists = new()
                {
                    Text = artists,
                    HorizontalAlignment = HorizontalAlignment.Left,
                    VerticalAlignment = VerticalAlignment.Center,
                    FontSize = 20,
                    Margin = new Thickness(800, 0, 0, 0)
                };
                TextBlock tblock_Time = new()
                {
                    Text = one.DuartionTime,
                    HorizontalAlignment = HorizontalAlignment.Right,
                    VerticalAlignment = VerticalAlignment.Center,
                    FontSize = 20
                };

                grids[index].Children.Add(label_index);
                grids[index].Children.Add(separator);
                grids[index].Children.Add(tblock_Name);
                grids[index].Children.Add(tblock_Artists);
                grids[index].Children.Add(tblock_Time);
                Songs.Items.Add(grids[index]);
            }
        }

        private void Play(object pramparameter)
        {
            Song song;
            try
            {
                song = new(pramparameter.ToString());
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
            for (int i = 0; i <= artist.Length - 1; i++)
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
            Res.res.CPlayAlbumPicUrl = song.CoverUrl;
            Res.res.CPlayAlbumId = song.AlbumId;
        }

        private void Songs_PreviewMouseDoubleClick(object sender, MouseButtonEventArgs e)
        {
            Grid currentSelected = (Grid)Songs.SelectedItem;
            Play(currentSelected.Tag);
        }
    }

    public class ViewModel
    {
        public ICommand Command => new PagePlaylistMethods(Play, CanExec);

        private bool isCanExec = true;

        private void Play(object pramparameter)
        {
            Song song;
            try
            {
                song = new(pramparameter.ToString());
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
            for (int i = 0; i <= artist.Length - 1; i++)
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
            Res.res.CPlayAlbumPicUrl = song.CoverUrl;
            Res.res.CPlayAlbumId = song.AlbumId;
            isCanExec = false;
        }

        private bool CanExec(object pramparameter)
        {
            return isCanExec;
        }
    }
}