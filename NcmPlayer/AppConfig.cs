using System.IO;

namespace NcmPlayer
{
    public static class AppConfig
    {
        public static string AppVersion = "0.6.15";

        public static string RegPath = @"HKEY_CURRENT_USER\SOFTWARE\Miaoywww\NcmPlayer\";

        public static string ApiUrl = "http://localhost:3000";

        // public static string MusicsDirectory = $"{AppDomain.CurrentDomain.SetupInformation.ApplicationBase}musics\\";

        public static string musicsDirectory;

        public static string MusicsDirectory
        {
            get
            {
                if (string.IsNullOrEmpty(musicsDirectory))
                {
                    if (Directory.Exists("D:\\"))
                    {
                        musicsDirectory = "D:\\NcmMusics\\";
                    }
                    else
                    {
                        musicsDirectory = "C:\\NcmMusics\\";
                    }
                }
                return musicsDirectory;
            }
        }

        public static string MusicsPath(string id, string type)
        {
            return $"{MusicsDirectory}{id}.{type}";
        }
    }
}