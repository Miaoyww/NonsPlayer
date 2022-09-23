using NcmApi;
using NcmPlayer.CloudMusic;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Effects;

namespace NcmPlayer.Views.Pages.Recommend
{
    /// <summary>
    /// RecommendSongs.xaml 的交互逻辑
    /// </summary>
    public partial class RecommendSongs : Page
    {
        public RecommendSongs()
        {
            InitializeComponent();
            DataContext = ResEntry.res;

            GetRecommend();
        }

        private UIElement lastMoveOn;
        private UIElement currentMoveOn;
        private bool storyboardLoaded = false;
        private bool mouseItemsMoving = false;
        private int clickCount = 0;

        public void GetRecommend()
        {
            Thread thread = new(() =>
            {
                Stopwatch sw = new();
                sw.Start();
                JObject temp = Api.Recommend.Songs(ResEntry.ncm);
                sw.Stop();
                Debug.WriteLine($"获取每日单曲耗时{sw.ElapsedMilliseconds}");
                JArray result = (JArray)temp["data"]["dailySongs"];

                sw.Restart();
                foreach (JObject item in result)
                {
                    Song one = new(item, true);
                    Debug.WriteLine($"当前加载歌曲: {one.Name}");
                    Dispatcher.BeginInvoke(new Action(() =>
                    {
                        Border parent = new()
                        {
                            Height = 80,
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

                        Border b_cover = new()
                        {
                            HorizontalAlignment = HorizontalAlignment.Left,
                            VerticalAlignment = VerticalAlignment.Center,
                            Margin = new Thickness(20, 0, 0, 0),
                            Width = 60,
                            Height = 60,
                            CornerRadius = new CornerRadius(5),
                            Effect = new DropShadowEffect() { Color = Color.FromArgb(60, 227, 227, 227), Opacity = 0.1 },
                        };
                        Task getCover = new Task(() =>
                        {
                            Stream coverStream = one.GetCover(50, 50);
                            Dispatcher.BeginInvoke(new Action(() =>
                            {
                                b_cover.Background = PublicMethod.ConvertBrush(coverStream);
                            }));
                        });
                        getCover.Start();
                        TextBlock tblock_Name = new()
                        {
                            Text = one.Name,
                            HorizontalAlignment = HorizontalAlignment.Left,
                            VerticalAlignment = VerticalAlignment.Center,
                            FontWeight = FontWeights.Bold,
                            FontSize = 20,
                            Margin = new Thickness(95, 0, 0, 0),
                            Width = 290,
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
                        content.Children.Add(b_cover);
                        content.Children.Add(tblock_Name);
                        content.Children.Add(tblock_Artists);
                        content.Children.Add(tblock_Time);
                        corner.Child = content;
                        parent.Child = corner;
                        SongList.Children.Add(parent);
                        SongList.Children.Add(new Separator() { BorderThickness = new Thickness(0), Height = 5 });
                    }));
                }
                sw.Stop();
                Debug.WriteLine($"更新歌曲耗时{sw.ElapsedMilliseconds}");
            });
            thread.IsBackground = true;
            thread.Start();
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
    }
}