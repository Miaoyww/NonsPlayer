using Microsoft.Win32;
using NcmPlayer.Framework.Model;
using NcmPlayer.Framework.Player;
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

            b_image.DataContext = ResEntry.musicPlayer;
            tblock_title.DataContext = ResEntry.musicPlayer;
            tblock_artists.DataContext = ResEntry.musicPlayer;
            slider_postion.DataContext = ResEntry.musicPlayer;
            label_currentTime.DataContext = ResEntry.musicPlayer;
            label_wholeTime.DataContext = ResEntry.musicPlayer;
        }

        private void Timer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                if (ResEntry.musicPlayer.IsPlaying)
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

        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.wholePlaylist.Last();
        }

        private void btn_play_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.musicPlayer.Play();
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
            var value = (e.GetPosition(slider_postion).X / slider_postion.ActualWidth) * (slider_postion.Maximum - slider_postion.Minimum);
            ResEntry.musicPlayer.SetPosition(value);
        }

        private void slider_postion_DragStarted(object sender, System.Windows.Controls.Primitives.DragStartedEventArgs e)
        {
            slider_postion.Maximum = ResEntry.musicPlayer.DurationTimeDouble;
            slider_postion.DataContext = null;
            isUser = true;
        }

        private void slider_postion_DragCompleted(object sender, System.Windows.Controls.Primitives.DragCompletedEventArgs e)
        {
            if (!ResEntry.musicPlayer.IsPlaying)
            {
                ResEntry.musicPlayer.Play();
            }
            ResEntry.musicPlayer.SetPosition(positionTemp);
            slider_postion.DataContext = ResEntry.musicPlayer;
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