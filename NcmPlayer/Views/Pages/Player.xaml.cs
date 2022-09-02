using Microsoft.Win32;
using NcmPlayer.CloudMusic;
using NcmPlayer.Resources;
using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Text;
using System.Threading;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using Brush = System.Windows.Media.Brush;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// Player.xaml 的交互逻辑
    /// </summary>
    public partial class Player : Page
    {
        public static Player playerPage;

        public class LrcVis
        {
            private TimeSpan showTime;
            private string lrcContent;
            private StackPanel vis;

            public TimeSpan ShowTime
            {
                get => showTime;
                set => showTime = value;
            }

            public string LrcContent
            {
                get => lrcContent;
                set => lrcContent = value;
            }

            public StackPanel Vis
            {
                get => vis;
                set => vis = value;
            }
        }

        public static System.Timers.Timer timer = new System.Timers.Timer();
        private bool isUser = false;
        private double positionTemp;
        public List<LrcVis> lrcVis = new List<LrcVis>();
        private bool lrcControling = false;
        private List<bool> mouseWheeling = new List<bool>();
        private int currentlrc = 0;

        public Player()
        {
            InitializeComponent();
            playerPage = this;
            DataContext = ResEntry.res;
            timer.Interval = 100;
            timer.Elapsed += Timer_Elapsed;
            timer.Start();

            b_image.DataContext = ResEntry.songInfo;
            tblock_title.DataContext = ResEntry.songInfo;
            tblock_artists.DataContext = ResEntry.songInfo;
            slider_postion.DataContext = ResEntry.songInfo;
            label_currentTime.DataContext = ResEntry.songInfo;
            label_wholeTime.DataContext = ResEntry.songInfo;

            try
            {
                ResEntry.songInfo.Volume = int.Parse(RegGeter.RegGet("Song", "SongVolume").ToString());
                ResEntry.songInfo.DurationTime = TimeSpan.Parse((string)RegGeter.RegGet("Song", "SongDurationTime"));
                ResEntry.songInfo.Postion = TimeSpan.Parse((string)RegGeter.RegGet("Song", "SongPostion"));
                ResEntry.songInfo.Name = (string)RegGeter.RegGet("Song", "SongName");
                ResEntry.songInfo.Artists = (string)RegGeter.RegGet("Song", "SongArtists");
                ResEntry.songInfo.Cover(new MemoryStream(Convert.FromBase64String((string)RegGeter.RegGet("Song", "SongCover"))));
                ResEntry.songInfo.FilePath = (string)RegGeter.RegGet("Song", "SongPath");
                ResEntry.songInfo.AlbumCoverUrl = (string)RegGeter.RegGet("Song", "SongAlbumUrl");
                
                string lrcString = Encoding.UTF8.GetString(Convert.FromBase64String((string)RegGeter.RegGet("Song", "SongLrc")));
                ResEntry.songInfo.LrcString = lrcString;
                UpdateLrc(new Lrcs(lrcString));
                // MusicPlayer.Init(ResEntry.res.CPlayPath);
                if (listview_lrc.Items.Count == 0)
                {
                    listview_lrc.Items.Add(getPanel("当前未播放音乐"));
                }
            }
            catch
            {
            }
            MusicPlayer.Reload();
        }

        private void Timer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                playerPage.ChangeVisLrc();
                if (ResEntry.songInfo.IsPlaying)
                {
                    if (btn_play.Icon != Wpf.Ui.Common.SymbolRegular.Pause24)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Pause24;
                    }
                }
                else
                {
                    if (btn_play.Icon != Wpf.Ui.Common.SymbolRegular.Play24)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Play24;
                    }
                }
            }));
        }

        public StackPanel getPanel(string content)
        {
            StackPanel panel = new();
            var bc = new BrushConverter();
            Label lrc = new()
            {
                Name = "lrcContent",
                Content = content,
                FontSize = 25,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
                Foreground = (Brush)bc.ConvertFrom("#FF9C9C9C"),
                FontWeight = FontWeights.Bold
            };
            Separator separator = new()
            {
                BorderThickness = new Thickness(0),
                Height = 20,
            };
            panel.Children.Add(lrc);
            // panel.Children.Add(separator);
            return panel;
        }

        public void ClearLrc()
        {
            listview_lrc.Items.Clear();
            lrcVis.Clear();
        }

        public void UpdateLrc(Lrcs lrcs)
        {
            foreach (Lrc item in lrcs.GetLrcs)
            {
                StackPanel vis = getPanel(item.GetLrc);
                LrcVis lrv = new LrcVis();
                vis.Tag = item.GetTime;
                vis.PreviewMouseDown += SelectedLrc;
                lrv.LrcContent = item.GetLrc;
                lrv.ShowTime = item.GetTime;
                lrv.Vis = vis;
                lrcVis.Add(lrv);
                listview_lrc.Items.Add(vis);
            }
            if (playerPage.lrcVis.Count >= 8)
            {
                for (int i = 0; i <= 8; i++)
                {
                    StackPanel vis = getPanel("");
                    listview_lrc.Items.Add(vis);
                }
            }
        }

        private void SelectedLrc(object sender, System.Windows.Input.MouseButtonEventArgs e)
        {
            MusicPlayer.Position(
                ((TimeSpan)((StackPanel)sender).Tag).TotalSeconds
                );
        }

        public void ChangeVisLrc()
        {
            if (listview_lrc.Items.Count > 1)
            {
                for (int index = 0; index < lrcVis.Count; index++)
                {
                    if (lrcVis.Count == 0)
                    {
                        break;
                    }
                    if (ResEntry.songInfo.Postion >= lrcVis[index].ShowTime && index + 1 < lrcVis.Count - 1 && ResEntry.songInfo.Postion < lrcVis[index + 1].ShowTime)
                    {
                        Label content = ((Label)((StackPanel)listview_lrc.Items[index]).Children[0]);
                        if (content.Content != "")
                        {
                            content.FontSize = 29;
                        }
                        var bc = new BrushConverter();
                        ((Label)((StackPanel)listview_lrc.Items[index]).Children[0]).Foreground = (Brush)bc.ConvertFromString("#ffffff");
                        if (!lrcControling)
                        {
                            currentlrc = index;
                            try
                            {
                                listview_lrc.ScrollIntoView(listview_lrc.Items[currentlrc + 8]);
                            }
                            catch
                            {

                            }
                        }
                        for (int i = 0; i <= lrcVis.Count; i++)
                        {
                            if (i != index)
                            {
                                if (listview_lrc.Items.Count - 8 >= i)
                                {
                                    ((Label)((StackPanel)listview_lrc.Items[i]).Children[0]).FontSize = 25;
                                    ((Label)((StackPanel)listview_lrc.Items[i]).Children[0]).Foreground = (Brush)bc.ConvertFromString("#FF9C9C9C");
                                }
                            }
                        }
                    }
                }
            }
        }

        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.wholePlaylist.Last();
        }

        private void btn_play_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            MusicPlayer.Play();
        }

        private void btn_next_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.wholePlaylist.Next();
        }

        private void btn_like_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            // PlayerMethods.SendMessage("like");
        }

        private void Window_Loaded(object sender, RoutedEventArgs e)
        {
        }

        private void SaveImageTo_Click(object sender, RoutedEventArgs e)
        {
            if (ResEntry.songInfo.AlbumCoverUrl != null)
            {
                SaveFileDialog sfd = new SaveFileDialog();
                sfd.Title = "选择保存至的位置";
                sfd.Filter = "图片| *.jpg;*.png";
                sfd.FileName = tblock_title.Text;
                if ((bool)sfd.ShowDialog())
                {
                    string path = sfd.FileName;
                    using (WebClient client = new WebClient())
                    {
                        client.DownloadFileAsync(new Uri(ResEntry.songInfo.AlbumCoverUrl), path);
                    }
                }
            }
        }

        private void btn_playerHide_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.ScreenControl();
        }

        private void slider_postion_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            if (isUser)
            {
                positionTemp = slider_postion.Value;
            }
        }

        private void slider_postion_PreviewMouseDown(object sender, System.Windows.Input.MouseEventArgs e)
        {
            // isUser = true;
            var value = (e.GetPosition(slider_postion).X / slider_postion.ActualWidth) * (slider_postion.Maximum - slider_postion.Minimum);
            MusicPlayer.Position(value);
        }

        private void slider_postion_DragStarted(object sender, System.Windows.Controls.Primitives.DragStartedEventArgs e)
        {
            slider_postion.Maximum = ResEntry.songInfo.DurationTimeDouble;
            slider_postion.DataContext = null;
            isUser = true;
        }

        private void slider_postion_DragCompleted(object sender, System.Windows.Controls.Primitives.DragCompletedEventArgs e)
        {
            if (!ResEntry.songInfo.IsPlaying)
            {
                MusicPlayer.Play();
            }
            MusicPlayer.Position(positionTemp);
            slider_postion.DataContext = ResEntry.songInfo;
            isUser = false;
        }

        private void listview_lrc_PreviewMouseDown(object sender, System.Windows.Input.MouseButtonEventArgs e)
        {
            lrcControling = true;
        }

        private void listview_lrc_PreviewMouseUp(object sender, System.Windows.Input.MouseButtonEventArgs e)
        {
            lrcControling = false;
        }

        private void listview_lrc_PreviewMouseWheel(object sender, System.Windows.Input.MouseWheelEventArgs e)
        {
            Thread wheeling = new(_ =>
            {
                lrcControling = true;
                mouseWheeling.Add(true);
                Thread.Sleep(2000);
                Dispatcher.BeginInvoke(new Action(() =>
                {
                    mouseWheeling.RemoveAt(0);
                    if (mouseWheeling.Count > 0)
                    {
                        lrcControling = true;
                    }
                    else
                    {
                        lrcControling = false;
                        try
                        {
                            listview_lrc.ScrollIntoView(listview_lrc.Items[currentlrc - 4]);
                        }
                        catch (ArgumentOutOfRangeException)
                        {
                            try
                            {
                                listview_lrc.ScrollIntoView(listview_lrc.Items[currentlrc]);

                            }
                            catch (ArgumentOutOfRangeException)
                            {

                            }
                        }
                    }
                }));
            });
            wheeling.IsBackground = true;
            wheeling.Start();
        }
    }
}