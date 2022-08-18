using Microsoft.Win32;
using NcmPlayer.Player;
using NcmPlayer.CloudMusic;
using System;
using System.Collections;
using System.ComponentModel;
using System.IO;
using System.Net;
using System.Security.Policy;
using System.Windows;
using System.Windows.Controls;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// Player.xaml 的交互逻辑
    /// </summary>
    public partial class Player : Page
    {
        public static Player playerPage;
        public static System.Timers.Timer timer = new System.Timers.Timer();
        public static System.Timers.Timer timerPostion = new System.Timers.Timer();
        private bool isPlaying = false;
        private bool isUser = false;

        public Player()
        {
            InitializeComponent();
            playerPage = this;
            DataContext = Res.res;
            timerPostion.Elapsed += TimerPostion_Elapsed;
            timerPostion.Interval = 800;
            timerPostion.Start();

            b_image.DataContext = Res.res;
            tblock_title.DataContext = Res.res;
            tblock_artists.DataContext = Res.res;
            slider_postion.DataContext = Res.res;
            label_currentTime.DataContext = Res.res;
            label_wholeTime.DataContext = Res.res;
        }

        private void TimerPostion_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                if (Res.res.IsPlaying)
                {
                    // slider_postion.Maximum = Res.res.currentPlayWholeTime;
                    // slider_postion.Value = Res.res.currentPlayPostion;
                    // label_currentTime.Content = Res.res.currentPlayPostionString;
                    // label_wholeTime.Content = Res.res.currentPlayWholeTimeString;
                }
            }));
        }

        private void Timer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                if (Res.res.IsPlaying)
                {
                    if (!isPlaying)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Pause24;
                        isPlaying = true;
                    }
                }
                else
                {
                    if (isPlaying)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Play24;
                        isPlaying = false;
                    }
                }
            }));
        }

        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            // PlayerMethods.SendMessage("last");
        }

        private void btn_play_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            MusicPlayer.Play();
        }

        private void btn_next_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            // PlayerMethods.SendMessage("next");
        }

        private void btn_like_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            // PlayerMethods.SendMessage("like");
        }

        private void Window_Loaded(object sender, RoutedEventArgs e)
        {
            /*
            Thread thread = new(() =>
            {
                // PlayerMethods.SendMessage("connect");
            });
            thread.IsBackground = true;
            thread.Start();*/
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
                MusicPlayer.Postion((int)slider_postion.Value);
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