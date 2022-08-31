using NcmPlayer.CloudMusic;
using NcmPlayer.Resources;
using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
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
        }

        private UIElement lastMoveOn;
        private UIElement currentMoveOn;
        private bool storyboardLoaded = false;
        private bool mouseItemsMoving = false;
        private int clickCount = 0;

        #region 属性及初始化

        public Border[] songBorderList;

        public List<Song> songlist = new List<Song>();

        public string Name
        {
            set => PlaylistName.Text = value;
        }

        public string CreateTime
        {
            set => PlaylistCreateTime.Text = $"· {value.Split(" ")[0]}";
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
            set => PlaylistCreator.Text = $"By {value}";
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
            int gridCount = songs.Length;
            songBorderList = new Border[songs.Length];
            for (int index = 0; index < gridCount; index++)
            {
                Song one = songs[index];
                songlist.Add(one);
                Border parent = new()
                {
                    Height = 40,
                    Tag = one.Id,
                    CornerRadius = new CornerRadius(10, 10, 10, 10),
                    HorizontalAlignment = HorizontalAlignment.Stretch,
                    Background = new SolidColorBrush(Color.FromArgb(100, 212, 212, 212))
                };
                parent.Background.Opacity = 0;
                parent.PreviewMouseMove += Parent_PreviewMouseMove;
                parent.PreviewMouseLeftButtonDown += Parent_PreviewMouseLeftButtonDown;
                Border corner = new()
                {
                    CornerRadius = new CornerRadius(10, 10, 10, 10),
                    Margin = new Thickness(0, 5, 0, 5)
                };
                Grid content = new();
                string artists = string.Empty;
                if (one.Artists[0] == string.Empty && one.Artists.Length == 1)
                {
                    artists = "未知艺人";
                }
                else
                {
                    for (int i = 0; i <= one.Artists.Length - 1; i++)
                    {
                        if (i != one.Artists.Length - 1)
                        {
                            artists += one.Artists[i] + " / ";
                        }
                        else
                        {
                            artists += one.Artists[i];
                        }
                    }
                }
                TextBlock tblock_Name = new()
                {
                    Text = one.Name,
                    HorizontalAlignment = HorizontalAlignment.Left,
                    VerticalAlignment = VerticalAlignment.Center,
                    FontWeight = FontWeights.Bold,
                    FontSize = 20,
                    Margin = new Thickness(20, 0, 0, 0),
                    Width = 330,
                    TextTrimming = TextTrimming.CharacterEllipsis
                };
                TextBlock tblock_Artists = new()
                {
                    Text = artists,
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center,
                    FontSize = 16,
                    Width = 200,
                    TextTrimming = TextTrimming.CharacterEllipsis
                };
                TextBlock tblock_Time = new()
                {
                    Text = one.DuartionTimeString,
                    HorizontalAlignment = HorizontalAlignment.Right,
                    VerticalAlignment = VerticalAlignment.Center,
                    FontSize = 14,
                    Margin = new Thickness(0, 0, 20, 0)
                };
                content.Children.Add(tblock_Name);
                content.Children.Add(tblock_Artists);
                content.Children.Add(tblock_Time);
                corner.Child = content;
                parent.Child = corner;
                songBorderList[index] = parent;
                Songs.Children.Add(songBorderList[index]);
                Songs.Children.Add(new Separator() { BorderThickness = new Thickness(0), Height = 5 });
            }
        }

        private void Parent_PreviewMouseLeftButtonDown(object sender, MouseButtonEventArgs e)
        {
            clickCount++;
            ThreadPool.QueueUserWorkItem(_ =>
            {
                Thread.Sleep(150);
                Dispatcher.BeginInvoke(new Action(() =>
                {
                    if (clickCount >= 2)
                    {
                        Play(((Border)sender).Tag.ToString());
                        clickCount = 0;
                    }
                    else
                    {
                        clickCount = 0;
                    }
                }));
            });
        }

        private void Parent_PreviewMouseMove(object sender, MouseEventArgs e)
        {
            mouseItemsMoving = true;
            if (lastMoveOn != (Border)sender)
            {
                if (lastMoveOn != null)
                {
                    Storyboard storyboardEnd = (Storyboard)Resources["MouseMoveOut"];
                    storyboardEnd.Begin((Border)lastMoveOn);
                }
                Storyboard storyboardStart = (Storyboard)Resources["MouseMoveOn"];
                storyboardStart.Begin((Border)sender);
                storyboardLoaded = true;
                lastMoveOn = (Border)sender;
            }
            mouseItemsMoving = false;
        }

        private void Play(object pramparameter)
        {
            PublicMethod.PagePlaylistBar.Play(pramparameter.ToString());
        }

        #endregion 属性及初始化

        private void Songs_PreviewMouseDoubleClick(object sender, MouseButtonEventArgs e)
        {
            /*
            Grid currentSelected = (Grid)Songs.SelectedItem;
            if (currentSelected != null)
            {
                Play(currentSelected.Tag);
            }*/
        }

        private void PlayAll_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.PagePlaylistBar.ClearSongs();
            PublicMethod.PagePlaylistBar.UpdateSongsList(songlist);
            ResEntry.wholePlaylist.Play(0);
        }

        private void Page_PreviewMouseMove(object sender, MouseEventArgs e)
        {
        }
    }
}