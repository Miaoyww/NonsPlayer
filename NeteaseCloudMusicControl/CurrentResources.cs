using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Controls;

namespace NeteaseCloudMusicControl
{
    public static class CurrentResources
    {
        public static MediaElement musicplayer;

        public static string soundPath;

        public static bool isPlayed = false;

        public static string CurrentVolume = string.Empty;

        public static string ServerIp = "127.0.0.1";

        public static string ServerPort = "4000";

        public static string Log = string.Empty;

        public static string SoundsPath = "./sounds.json";
    }
}
