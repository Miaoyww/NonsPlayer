using Microsoft.Win32;
using NcmPlayer.Resources;
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
            DataContext = ResEntry.res;
        }

        private void OnChangeTheme(bool isChecked)
        {
            switch (isChecked)
            {
                case true:
                    if (ResEntry.res.CurrentTheme == ThemeType.Light)
                       break;

                    Theme.Apply(ThemeType.Light);
                    ResEntry.res.CurrentTheme = ThemeType.Light;

                    break;

                default:
                    if (ResEntry.res.CurrentTheme == ThemeType.Dark)
                        break;

                    Theme.Apply(ThemeType.Dark);
                    ResEntry.res.CurrentTheme = ThemeType.Dark;

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
            switch (ResEntry.res.CurrentTheme)
            {
                case ThemeType.Light:
                    tgs_theme.IsChecked = true;
                    break;
                case ThemeType.Dark:
                    tgs_theme.IsChecked = false;
                    break;
            }
        }

        private void tgs_theme_Click(object sender, RoutedEventArgs e)
        {
            OnChangeTheme((bool)tgs_theme.IsChecked);
        }
    }
}