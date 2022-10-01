using NcmApi;
using NcmPlayer.Framework.Model;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System;
using System.IO;
using System.Threading;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Effects;

namespace NcmPlayer.Views.Pages
{
    public partial class Home : Page
    {
        public Home()
        {
            InitializeComponent();
            DataContext = ResEntry.res;
            UpdateTopPlaylist();

            b_dailyMusic.DataContext = ResEntry.res;
        }

        private StackPanel getStackPanel(string title, string id, Stream cover)
        {
            var stackPanel = new StackPanel()
            {
                Focusable = false,
                Margin = new Thickness(0, 0, 20, 0),
            };
            var borderCover = new Border()
            {
                Width = 200,
                Height = 200,
                Tag = id,
                Cursor = System.Windows.Input.Cursors.Hand,
                CornerRadius = new CornerRadius(20),
                Effect = new DropShadowEffect() { Color = Color.FromArgb(60, 227, 227, 227), Opacity = 0.1 },
                Background = PublicMethod.ConvertBrush(cover)
            };
            borderCover.PreviewMouseLeftButtonDown += OpenNewPlaylist;
            var tblockTitle = new TextBlock()
            {
                Text = title,
                TextAlignment = TextAlignment.Left,
                Margin = new Thickness(0, 15, 0, 0),
                FontWeight = FontWeights.Bold,
                FontSize = 20,
                Width = 200,
                Height = 94,
                TextWrapping = TextWrapping.WrapWithOverflow,
                Padding = new Thickness(0, 0, 0, 0)
            };
            stackPanel.Children.Add(borderCover);
            stackPanel.Children.Add(tblockTitle);
            return stackPanel;
        }

        private void OpenNewPlaylist(object sender, System.Windows.Input.MouseButtonEventArgs e)
        {
            Tool.OpenPlayListDetail(long.Parse(((Border)sender).Tag.ToString()));
        }

        public void UpdateTopPlaylist()
        {
            Thread mainThread = new(() =>
            {
                JObject response = Api.Playlist.Personalized(ResEntry.ncm, 20).Result;
                if ((int)response["code"] == 200)
                {
                    JArray playlists = (JArray)response["result"];
                    int count = 0;
                    foreach (JObject item in playlists)
                    {
                        Thread getPlaylist = new Thread(_ =>
                        {
                            Stream playlistCover = HttpRequest.StreamHttpGet(item["picUrl"].ToString() + "?param=180y180").Result;
                            this.Dispatcher.BeginInvoke(new Action(() =>
                            {
                                StackPanel playlistView = getStackPanel(
                                (string)item["name"], item["id"].ToString(), playlistCover);
                                panel_nicePlaylists.Children.Add(playlistView);
                                count++;
                            }));
                        })
                        {
                            IsBackground = true
                        };
                        getPlaylist.Start();
                    }
                }
            });
            mainThread.IsBackground = true;
            mainThread.Start();
        }

        private void b_dailyMusic_PreviewMouseLeftButtonDown(object sender, System.Windows.Input.MouseButtonEventArgs e)
        {
            if (ResEntry.res.recommendMusics == null)
            {
                ResEntry.res.recommendMusics = new();
            }
            PublicMethod.ChangePage(ResEntry.res.recommendMusics);
        }
    }
}