using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace NeteaseCloudMusicControl
{
    public static class AppConfig
    {
        public static string AppVersion = "0.1.5";

        public static string RegPath = @"HKEY_CURRENT_USER\SOFTWARE\Miaoywww\NeteaseCloudMusicControl\";

        public static string ApiUrl = "http://localhost:3000";

        public static string SongsDirectory = $"{AppDomain.CurrentDomain.SetupInformation.ApplicationBase}songs\\";

        public static string SongsPath(string id, string type)
        {
            return $"{SongsDirectory}{id}.{type}";
        }

        public static CurrentResources resources;
    }
}
