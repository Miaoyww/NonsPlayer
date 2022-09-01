using System;

namespace NcmPlayer
{
    public static class AppConfig
    {
        public static string AppVersion = "0.5.6";

        public static string RegPath = @"HKEY_CURRENT_USER\SOFTWARE\Miaoywww\NcmPlayer\";

        public static string ApiUrl = "http://localhost:3000";

        public static string SongsDirectory = $"{AppDomain.CurrentDomain.SetupInformation.ApplicationBase}songs\\";

        public static string SongsPath(string id, string type)
        {
            return $"{SongsDirectory}{id}.{type}";
        }
    }
}