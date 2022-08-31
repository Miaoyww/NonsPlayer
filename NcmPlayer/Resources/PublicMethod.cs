using NcmPlayer.Views;
using NcmPlayer.Views.Pages;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;
using Wpf.Ui.Appearance;
using Wpf.Ui.Common;
using Wpf.Ui.Controls;

namespace NcmPlayer.Resources
{
    // 用于储存操作主界面的一些方法与变量
    public static class PublicMethod
    {
        public static Frame Screenframe;
        public static Frame Pageframe;
        public static Frame Playlistbarframe;

        public static Page LastPage;
        public static Home PageHome = new();
        public static Settings PageSettings;
        public static Explore PageExplore;
        public static Login PageLogin = new();
        public static Player PagePlayer = new();
        public static PlaylistBar PagePlaylistBar = new();
        public static string CurrentPage = string.Empty;

        // 由于大部分的ui都在MainWindow中，在此传递以便初始化
        public static void Init(MainWindow window)
        {
            MusicPlayer.InitPlayer();
            Theme.Apply(ResEntry.res.CurrentTheme);
            Screenframe = window.ScreenFrame;
            Pageframe = window.PageFrame;
            Playlistbarframe = window.PlayListBar;

            window.PageFrame.Content = PageHome;
            window.ScreenFrame.Content = PagePlayer;
            window.PlayListBar.Content = PagePlaylistBar;

        }

        public static void ShowDialog(string content, string title)
        {
            MainWindow.acc.dialog.Visibility = Visibility.Visible;
            MainWindow.acc.dialog.Title = title;
            MainWindow.acc.dialog.Content = content;
            MainWindow.acc.dialog.ButtonLeftVisibility = Visibility.Visible;
            MainWindow.acc.dialog.ButtonRightVisibility = Visibility.Visible;
            MainWindow.acc.dialog.Show();
        }

        public static void ShowDialog(object content)
        {
            MainWindow.acc.dialog.Visibility = Visibility.Visible;
            MainWindow.acc.dialog.Title = "";
            MainWindow.acc.dialog.Content = content;
            MainWindow.acc.dialog.ButtonLeftVisibility = Visibility.Hidden;
            MainWindow.acc.dialog.ButtonRightVisibility = Visibility.Hidden;
            MainWindow.acc.dialog.Background = null;
            MainWindow.acc.dialog.Show();
        }

        public static void SnackLog(string content, string title, int TimeOut = 1000, SymbolRegular icon = SymbolRegular.Info24)
        {
            if (!MainWindow.acc.snackLog.IsShown)
            {
                MainWindow.acc.snackLog.Timeout = TimeOut;
                MainWindow.acc.snackLog.Visibility = Visibility.Visible;
                MainWindow.acc.snackLog.Title = title;
                MainWindow.acc.snackLog.Message = content;
                MainWindow.acc.snackLog.Icon = icon;
                MainWindow.acc.snackLog.Show();
            }
        }

        public static void HideDialog()
        {
            MainWindow.acc.dialog.Hide();
        }

        public static void ChangePage(Page page)
        {
            if (!CurrentPage.Equals(page.Title.ToString()))
            {
                if (Screenframe.Visibility == Visibility.Visible)
                {
                    ScreenControl();
                }
                Pageframe.Content = page;
                CurrentPage = page.Title;
            }
        }

        public static void ScreenControl()
        {
            ResEntry.res.ScreenSize = new double[] { Screenframe.RenderSize.Width, Screenframe.RenderSize.Height };
            ResEntry.res.PageSize = new double[] { Pageframe.RenderSize.Width, Pageframe.RenderSize.Height };
            if (!ResEntry.res.isShowingPlayer)
            {
                if (Screenframe.Visibility == Visibility.Visible)
                {
                    Screenframe.RenderTransform = new TranslateTransform(0, 0);
                    Storyboard story = (Storyboard)MainWindow.acc.Resources["Hide"];
                    story.Completed += delegate
                    {
                        Screenframe.Visibility = Visibility.Hidden;
                        ResEntry.res.isShowingPlayer = false;
                    };
                    story.Begin(Screenframe);
                }
                else
                {
                    Screenframe.RenderTransform = new TranslateTransform(0, 0);
                    Storyboard story = (Storyboard)MainWindow.acc.Resources["Show"];
                    story.Completed += delegate
                    {
                        ResEntry.res.isShowingPlayer = false;
                    };
                    story.Begin(Screenframe);
                    Screenframe.Visibility = Visibility.Visible;
                }
            }
        }

        public static void PlaylistBarControl()
        {
            if (Playlistbarframe.Visibility == Visibility.Visible)
            {
                Playlistbarframe.RenderTransform = new TranslateTransform(0, 0);
                Storyboard story = (Storyboard)MainWindow.acc.Resources["Hide"];
                story.Completed += delegate
                {
                    Playlistbarframe.Visibility = Visibility.Hidden;
                };
                story.Begin(Playlistbarframe);
            }
            else
            {
                Playlistbarframe.RenderTransform = new TranslateTransform(0, 0);
                Storyboard story = (Storyboard)MainWindow.acc.Resources["Show"];
                story.Begin(Playlistbarframe);
                Playlistbarframe.Visibility = Visibility.Visible;
            }
        }

        public static Brush ConvertBrush(Stream stream)
        {
            BitmapImage image = new();
            image.BeginInit();
            image.StreamSource = stream;
            image.EndInit();
            return new ImageBrush(image);
        }
    }
}