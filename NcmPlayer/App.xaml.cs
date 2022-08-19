using Microsoft.Win32;
using NcmPlayer.Player;
using Newtonsoft.Json.Linq;
using System.Collections.Generic;
using System.Net;
using System.Windows;
using System.Windows.Controls;
using Wpf.Ui.Appearance;

namespace NcmPlayer
{
    public partial class App : Application
    {
        private void Application_Startup(object sender, StartupEventArgs e)
        {
            JObject keys = new();
            keys.Add("ServerPort", Res.res.serverPort);
            keys.Add("CurrentVolume", "100");
            keys.Add("Theme", "Light");
            IEnumerable<JProperty> properties = keys.Properties();
            foreach (JProperty key in properties)
            {
                if (Registry.GetValue(AppConfig.RegPath, key.Name, null) == null)
                {
                    Registry.SetValue(AppConfig.RegPath, key.Name, key.Value);
                }
            }
            Res.res.serverPort = Registry.GetValue(AppConfig.RegPath, "ServerPort", Res.res.serverPort).ToString();
            Res.res.CVolume = int.Parse(Registry.GetValue(AppConfig.RegPath, "CurrentVolume", "100").ToString());
            MusicPlayer.InitPlayer();
            switch (Registry.GetValue(AppConfig.RegPath, "Theme", "Light").ToString()){
                case "Light":
                    Res.res.CurrentTheme = ThemeType.Light; 
                    break;

                case "Dark":
                    Res.res.CurrentTheme = ThemeType.Dark;
                    break;
            }

        }
    }
}