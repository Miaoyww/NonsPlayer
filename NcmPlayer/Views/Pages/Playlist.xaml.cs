using NcmPlayer.Framework.Model;
using NcmPlayer.Resources;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Effects;
using System.Windows.Media.Imaging;
using Button = Wpf.Ui.Controls.Button;
using PlayList = NcmPlayer.Framework.Model.PlayList;

namespace NcmPlayer.Views.Pages
{
    /// <summary>music
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
        private bool mouseIsMoving = false;
        private int clickCount = 0;
        private int loadedViewCount = 0;

        private PlayList playList;
        private List<Music> InMusics;

        #region 属性及初始化

        public List<Music> musiclist = new List<Music>();

        public string Name
        {
            set
            {
                PlaylistName.Text = value;
                PlaylistName.ToolTip = value;
            }
        }

        public string CreateTime
        {
            set => PlaylistCreateTime.Text = $"· {value.Split(" ")[0]}";
        }

        public string MusicsCount
        {
            set => PlaylistMusicsCount.Text = $"{value} 首歌曲";
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

        private async Task createAndUpdate(Music one, int index)
        {
            musiclist.Add(one);
            Border b_cover = new()
            {
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
                Margin = new Thickness(20, 0, 0, 0),
                Width = 40,
                Height = 40,
                CornerRadius = new CornerRadius(5),
                Effect = new DropShadowEffect() { Color = Color.FromArgb(60, 227, 227, 227), Opacity = 0.1 },
            };

            Task.Run(() =>
            {
                Stream coverStream = one.GetPic(30, 30).Result;
                Dispatcher.BeginInvoke(new Action(() =>
                {
                    b_cover.Background = PublicMethod.ConvertBrush(coverStream);
                }));
            });
            Border parent = new()
            {
                Height = 50,
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
            string artists = one.ArtistsName;
            if (string.IsNullOrEmpty(one.ArtistsName))
            {
                artists = "未知艺人";
            }
            StackPanel musicInfo = new()
            {
                Margin = new Thickness(75, 0, 0, 0),
                Width = 400,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
            };
            TextBlock tblock_Name = new()
            {
                Text = one.Name,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
                FontWeight = FontWeights.Bold,
                FontSize = 17,
                Width = 390,
                TextTrimming = TextTrimming.CharacterEllipsis
            };
            TextBlock tblock_Artists = new()
            {
                Text = artists,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 16,
                Width = 390,
                TextTrimming = TextTrimming.CharacterEllipsis
            };
            musicInfo.Children.Add(tblock_Name);
            musicInfo.Children.Add(tblock_Artists);
            TextBlock tblock_Time = new()
            {
                Text = one.DuartionTimeString,
                HorizontalAlignment = HorizontalAlignment.Right,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14,
                Margin = new Thickness(0, 0, 20, 0)
            };

            content.Children.Add(b_cover);
            content.Children.Add(musicInfo);
            content.Children.Add(tblock_Time);
            corner.Child = content;
            parent.Child = corner;
            Musics.Children.Add(parent);
            Musics.Children.Add(new Separator() { BorderThickness = new Thickness(0), Height = 5 });
        }

        private async Task updateList(int start, int end)
        {
            Stopwatch sw = new();
            sw.Start();
            if (start != 0)
            {
                Musics.Children.RemoveAt(Musics.Children.Count - 1);
            }
            end = end - 1;
            for (int index = start; index <= end; index++)
            {
                await createAndUpdate(InMusics[index], index);
                if (index == end)
                {
                    if (end < playList.MusicsCount - 1)
                    {
                        Button loadmore = new()
                        {
                            Margin = new Thickness(0, 10, 0, 10),
                            BorderThickness = new Thickness(1),
                            HorizontalAlignment = HorizontalAlignment.Center,
                            VerticalAlignment = VerticalAlignment.Center,
                            Padding = new Thickness(0),
                            FontSize = 18,
                            Width = 100,
                            Height = 40,
                            Opacity = 0.95,
                            Content = "加载更多"
                        };
                        loadmore.Click += Loadmore_Click;
                        Musics.Children.Add(loadmore);
                        sw.Stop();
                        Debug.WriteLine($"Playlist: UpdateMusicsList耗时{sw.ElapsedMilliseconds}");
                    }
                }
            }
        }

        private async void Loadmore_Click(object sender, RoutedEventArgs e)
        {
            int loadCount = 200;

            int goload = loadedViewCount + loadCount;
            if (goload > InMusics.Count)
            {
                if (goload < playList.MusicsCount)
                {
                    InMusics = InMusics.Concat(playList.InitArtWorkList(loadedViewCount, goload).Result.ToList()).ToList();
                }
                else
                {
                    goload = InMusics.Count;
                }
            }
            await updateList(loadedViewCount + 1, goload);
            loadedViewCount = goload;
        }

        public async Task UpdateMusicsList(Music[] musics, PlayList list)
        {
            playList = list;
            InMusics = musics.ToList();
            if (playList.MusicsCount >= 100)
            {
                loadedViewCount = 100;
            }
            else
            {
                loadedViewCount = playList.MusicsCount;
            }
            updateList(0, loadedViewCount);
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
                    currentMoveOn = (Border)sender;
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
            PublicMethod.PagePlaylistBar.Play(long.Parse(pramparameter.ToString()));
        }

        #endregion 属性及初始化

        private void Musics_PreviewMouseDoubleClick(object sender, MouseButtonEventArgs e)
        {

        }

        private void PlayAll(object sender, MouseButtonEventArgs e)
        {
            PublicMethod.PagePlaylistBar.ClearMusics();
            PublicMethod.PagePlaylistBar.UpdateMusicsList(musiclist);
            ResEntry.wholePlaylist.Play(0);
        }

        private void Page_PreviewMouseMove(object sender, MouseEventArgs e)
        {
            if (!mouseIsMoving)
            {
                mouseIsMoving = true;
            }
        }

        private void PlaylistCover_PreviewMouseMove(object sender, MouseEventArgs e)
        {
        }
    }
}