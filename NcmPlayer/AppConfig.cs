using System.IO;

namespace NcmPlayer
{
    public static class AppConfig
    {
        public static string AppVersion = "0.6.15";

        public static string RegPath = @"HKEY_CURRENT_USER\SOFTWARE\Miaoywww\NcmPlayer\";

        public static string ApiUrl = "http://localhost:3000";

        // public static string SongsDirectory = $"{AppDomain.CurrentDomain.SetupInformation.ApplicationBase}songs\\";

        public static string songsDirectory;

        public static string SongsDirectory
        {
            get
            {
                if (string.IsNullOrEmpty(songsDirectory))
                {
                    if (Directory.Exists("D:\\"))
                    {
                        songsDirectory = "D:\\NcmSongs\\";
                    }
                    else
                    {
                        songsDirectory = "C:\\NcmSongs\\";
                    }
                }
                return songsDirectory;
            }
        }

        public static string SongsPath(string id, string type)
        {
            return $"{SongsDirectory}{id}.{type}";
        }
    }
}