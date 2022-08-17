using Microsoft.Win32;
using NcmPlayer;
using NeteaseCloudMusicControl;
using Newtonsoft.Json.Linq;
using System.Collections.Generic;
using System.Net;
using System.Windows;
using System.Windows.Controls;

namespace NcmPlayer
{
    public partial class App : Application
    {
        private void Application_Startup(object sender, StartupEventArgs e)
        {
            JObject keys = new();
            keys.Add("ServerPort", Res.res.serverPort);
            keys.Add("CurrentVolume", "100");
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
        }
    }
}