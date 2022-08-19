using Microsoft.Win32;
using NcmPlayer;
using System.Windows;
using System.Windows.Controls;
using Wpf.Ui.Appearance;
using Wpf.Ui.Controls;

namespace NcmPlayer.Views.Pages
{
    public partial class Settings : Page
    {
        private Wpf.Ui.Appearance.ThemeType _currentTheme = Wpf.Ui.Appearance.ThemeType.Unknown;
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
            DataContext = Res.res;
        }

        private void OnChangeTheme(bool isChecked)
        {
            switch (isChecked)
            {
                case true:
                    if (Res.res.CurrentTheme == ThemeType.Light)
                       break;

                    Theme.Apply(ThemeType.Light);
                    Res.res.CurrentTheme = ThemeType.Light;

                    break;

                default:
                    if (Res.res.CurrentTheme == ThemeType.Dark)
                        break;

                    Theme.Apply(ThemeType.Dark);
                    Res.res.CurrentTheme = ThemeType.Dark;

                    break;
            }
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

        private void tgs_theme_Click(object sender, RoutedEventArgs e)
        {
            OnChangeTheme((bool)tgs_theme.IsChecked);
        }
    }
}