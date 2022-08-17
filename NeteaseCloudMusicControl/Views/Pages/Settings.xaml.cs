using Microsoft.Win32;
using NeteaseCloudMusicControl;
using System.Windows;
using System.Windows.Controls;
using Wpf.Ui.Controls;

namespace NcmPlayer.Views.Pages
{
    public partial class Settings : Page
    {
        public Settings settingsPage;
        private string IpAddress = "ServerIpAddress";
        private string Port = "ServerPort";
        private string ip;
        private string port;
        private bool isUser = false;

        public Settings()
        {
            InitializeComponent();
            settingsPage = this;
        }

        private void RegEditer(string key, object value)
        {
            Registry.SetValue(AppConfig.RegPath, key, value);
        }

        private void Page_Loaded(object sender, RoutedEventArgs e)
        {
            tblock_Version.Text = $"v{AppConfig.AppVersion}";
            isUser = true;
        }
    }
}