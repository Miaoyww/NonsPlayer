using NcmPlayer.Resources;
using System.Windows;
using System.Windows.Controls;
using Wpf.Ui.Appearance;

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

                    PublicMethod.ChangeTheme(ThemeType.Light);

                    break;

                default:
                    if (ResEntry.res.CurrentTheme == ThemeType.Dark)
                        break;

                    PublicMethod.ChangeTheme(ThemeType.Dark);

                    break;
            }
        }

        private void Page_Loaded(object sender, RoutedEventArgs e)
        {
            tblock_Version.Text = $"v{AppConfig.AppVersion}";
            switch (ResEntry.res.CurrentTheme)
            {
                case ThemeType.Light:
                    tgs_theme.IsChecked = true;
                    break;

                case ThemeType.Dark:
                    tgs_theme.IsChecked = false;
                    break;
            }
            tgs_dailySignin.IsChecked = bool.Parse(RegGeter.RegGet("DailySignin").ToString());
            isUser = true;

            ResEntry.res.allowDailySignin = (bool)tgs_dailySignin.IsChecked;
        }

        private void tgs_theme_Click(object sender, RoutedEventArgs e)
        {
            OnChangeTheme((bool)tgs_theme.IsChecked);
        }

        private void Button_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.SnackLog("Test", "Test");
        }

        private void tgs_dailySignin_Click(object sender, RoutedEventArgs e)
        {
            ResEntry.res.allowDailySignin = (bool)tgs_dailySignin.IsChecked;
            Regediter.Regedit("DailySignin", ResEntry.res.allowDailySignin);
        }
    }
}