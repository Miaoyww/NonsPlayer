using Microsoft.Win32;
using System.Windows;
using System.Windows.Controls;
using Wpf.Ui.Controls;

namespace NeteaseCloudMusicControl.Views.Pages
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
            CurrentResources.ServerIp = tbox_ipaddress.Text;
            CurrentResources.ServerPort = tbox_port.Text;
        }

        private void Page_Loaded(object sender, RoutedEventArgs e)
        {
            tblock_Version.Text = $"v{AppConfig.AppVersion}";
            tbox_ipaddress.Text = CurrentResources.ServerIp;
            tbox_port.Text = CurrentResources.ServerPort;
            isUser = true;
        }

        private void tbox_port_TextChanged(object sender, TextChangedEventArgs e)
        {
            if (isUser)
            {
                if (tbox_port.Text != "")
                {
                    if (int.Parse(tbox_port.Text) > 0 && int.Parse(tbox_port.Text) <= 65535)
                    {
                        port = tbox_port.Text;
                        RegEditer(Port, tbox_port.Text);
                    }
                    else
                    {
                        tbox_port.Text = "4000";
                    }
                }
            }
        }

        private void tbox_ipaddress_TextChanged(object sender, TextChangedEventArgs e)
        {
            if (isUser)
            {
                ip = tbox_ipaddress.Text;
                RegEditer(IpAddress, tbox_ipaddress.Text);
            }
        }
    }
}