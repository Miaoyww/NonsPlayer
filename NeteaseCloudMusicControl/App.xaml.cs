using Microsoft.Win32;
using NeteaseCloudMusicControl.Views.Methods;
using Newtonsoft.Json.Linq;
using System.Collections.Generic;
using System.Net;
using System.Windows;
using System.Windows.Controls;

namespace NeteaseCloudMusicControl
{
    public partial class App : Application
    {
        private void Application_Startup(object sender, StartupEventArgs e)
        {
            JObject keys = new();
            keys.Add("ServerIpAddress", "127.0.0.1");
            keys.Add("ServerPort", "4000");
            keys.Add("CurrentVolume", "100");
            IEnumerable<JProperty> properties = keys.Properties();
            foreach (JProperty key in properties)
            {
                if (Registry.GetValue(AppConfig.RegPath, key.Name, null) == null)
                {
                    Registry.SetValue(AppConfig.RegPath, key.Name, key.Value);
                }
            }
            CurrentResources.ServerIp = Registry.GetValue(AppConfig.RegPath, "ServerIpAddress", "127.0.0.1").ToString();
            CurrentResources.ServerPort = Registry.GetValue(AppConfig.RegPath, "ServerPort", "4000").ToString();
            CurrentResources.CurrentVolume = Registry.GetValue(AppConfig.RegPath, "CurrentVolume", "100").ToString();
            CurrentResources.musicplayer = new();
            CurrentResources.musicplayer.UnloadedBehavior = MediaState.Manual;
            CurrentResources.musicplayer.LoadedBehavior = MediaState.Manual;
            CurrentResources.musicplayer.MediaOpened += PlayerMethods.Musicplayer_MediaOpened;
            CurrentResources.musicplayer.MediaFailed += PlayerMethods.Musicplayer_MediaFailed;
            CurrentResources.musicplayer.MediaEnded += PlayerMethods.Musicplayer_MediaEnded;
            CurrentResources.musicplayer.Visibility = Visibility.Hidden;
            PlayerMethods.timer = new();
            PlayerMethods.timer.Elapsed += PlayerMethods.Timer_Elapsed;
        }

    }
}