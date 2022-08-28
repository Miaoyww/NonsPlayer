using Microsoft.Win32;
using NcmPlayer.CloudMusic;
using NcmPlayer.Player;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Text;
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
        public List<LrcVis> lrcVis = new List<LrcVis>();

        public Player()
        {
            InitializeComponent();
            playerPage = this;
            DataContext = Res.res;
            timer.Interval = 100;
            timer.Elapsed += Timer_Elapsed;
            timer.Start();

            b_image.DataContext = Res.res;
            tblock_title.DataContext = Res.res;
            tblock_artists.DataContext = Res.res;
            slider_postion.DataContext = Res.res;
            label_currentTime.DataContext = Res.res;
            label_wholeTime.DataContext = Res.res;

            Res.res.CPlayPostion = double.Parse((string)RegGeter.RegGet("Song", "SongPostion"));
            Res.res.CPlayName = (string)RegGeter.RegGet("Song", "SongName");
            Res.res.CPlayArtists = (string)RegGeter.RegGet("Song", "SongArtists");
            Res.res.Cover(new MemoryStream(Convert.FromBase64String((string)RegGeter.RegGet("Song", "SongCover"))));
            Res.res.CPlayPath = (string)RegGeter.RegGet("Song", "SongPath");
            Res.res.CPlayAlbumPicUrl = (string)RegGeter.RegGet("Song", "SongAlbumUrl");
            UpdateLrc(new Lrcs(Encoding.UTF8.GetString(Convert.FromBase64String((string)RegGeter.RegGet("Song", "SongLrc")))));
            MusicPlayer.Init(Res.res.CPlayPath);
            if (listview_lrc.Items.Count == 0)
            {
                listview_lrc.Items.Add(getPanel("当前未播放音乐"));
            }
        }

        private void Timer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                Views.Pages.Player.playerPage.ChangeVisLrc();
                if (Res.res.IsPlaying)
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
                FontSize = 30,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
                Foreground = (Brush)bc.ConvertFrom("#FF9C9C9C"),
                FontWeight = FontWeights.Bold
            };
            Separator separator = new()
            {
                BorderThickness = new Thickness(0),
                Height = 30,
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
                lrv.LrcContent = item.GetLrc;
                lrv.ShowTime = item.GetTime;
                lrv.Vis = vis;
                lrcVis.Add(lrv);
                listview_lrc.Items.Add(vis);
            }
        }

        public void ChangeVisLrc()
        {
            if (listview_lrc.Items.Count > 1)
            {
                for (int index = 0; index < lrcVis.Count; index++)
                {
                    if (lrcVis.Count != 0)
                    {
                        if (Res.res.CPlayPostion >= lrcVis[index].ShowTime.TotalSeconds && index + 1 < lrcVis.Count - 1 && Res.res.CPlayPostion < lrcVis[index + 1].ShowTime.TotalSeconds)
                        {
                            Label content = ((Label)((StackPanel)listview_lrc.Items[index]).Children[0]);
                            if (content.Content != "")
                            {
                                content.FontSize = 35;
                            }
                            var bc = new BrushConverter();
                            ((Label)((StackPanel)listview_lrc.Items[index]).Children[0]).Foreground = (Brush)bc.ConvertFromString("#ffffff");
                            listview_lrc.ScrollIntoView(listview_lrc.Items[index + 8]);
                            for (int i = 0; i <= lrcVis.Count; i++)
                            {
                                if (i != index)
                                {
                                    if (listview_lrc.Items.Count - 8 >= i)
                                    {
                                        ((Label)((StackPanel)listview_lrc.Items[i]).Children[0]).FontSize = 30;
                                        ((Label)((StackPanel)listview_lrc.Items[i]).Children[0]).Foreground = (Brush)bc.ConvertFromString("#FF9C9C9C");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            Res.wholePlaylist.Last();
        }

        private void btn_play_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            MusicPlayer.Play();
        }

        private void btn_next_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            Res.wholePlaylist.Next();
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
            if (Res.res.CPlayAlbumPicUrl != null)
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
                        client.DownloadFileAsync(new Uri(Res.res.CPlayAlbumPicUrl), path);
                    }
                }
            }
        }

        private void btn_playerHide_Click(object sender, RoutedEventArgs e)
        {
            MainWindow.ScreenControl();
        }

        private void slider_postion_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            if (isUser)
            {
                MusicPlayer.Postion(slider_postion.Value);
            }
        }

        private void slider_postion_PreviewMouseDown(object sender, System.Windows.Input.MouseEventArgs e)
        {
            isUser = true;
        }

        private void slider_postion_PreviewMouseUp(object sender, System.Windows.Input.MouseButtonEventArgs e)
        {
            isUser = false;
        }
    }
}