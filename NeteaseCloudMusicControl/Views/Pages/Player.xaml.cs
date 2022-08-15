using Microsoft.Win32;
using NeteaseCloudMusicControl.Views.Methods;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
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

namespace NeteaseCloudMusicControl.Views.Pages
{
    /// <summary>
    /// Player.xaml 的交互逻辑
    /// </summary>
    public partial class Player : Page
    {
        public static Player playerPage;
        public static System.Timers.Timer timer = new System.Timers.Timer();
        private bool isPlayed = false;
        public Player()
        {
            InitializeComponent();
            playerPage = this;
            timer.Interval = 100;
            timer.Elapsed += Timer_Elapsed;
            timer.Start();
        }

        private void Timer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                if (CurrentResources.isPlayed)
                {
                    if (!isPlayed)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Pause24;
                        isPlayed = true;
                    }
                }
                else
                {
                    if (isPlayed)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Play24;
                        isPlayed = false;
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
            PlayerMethods.Play();
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
            SaveFileDialog sfd = new SaveFileDialog();
            sfd.Title = "选择保存至的位置";
            sfd.Filter = "图片| *.jpg;*.png";
            sfd.FileName = tblock_title.Text;
            if ((bool)sfd.ShowDialog())
            {
                string path = sfd.FileName;
                StreamWriter streamWriter = new(path);
                streamWriter.Write(PlayerMethods.imageStream.ReadByte());
                streamWriter.Flush();
                streamWriter.Close();
            }

        }

        private void btn_playerHide_Click(object sender, RoutedEventArgs e)
        {
            MainWindow.ScreenControl();
        }
    }
}
