using Microsoft.Win32;
using NcmPlayer.Player;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System.Collections.Generic;
using Wpf.Ui.Appearance;

namespace NcmPlayer
{
    public static class OnloadFunc
    {
        public static void Load()
        {
            RegRegistered();

        }

        private static void RegRegistered()
        {
            IEnumerable<JProperty> properties;

            #region 基础路径

            JObject basePath = new();
            basePath.Add("ServerPort", Res.res.serverPort);
            basePath.Add("CurrentVolume", "100");
            basePath.Add("Theme", "Light");
            properties = basePath.Properties();
            foreach (JProperty key in properties)
            {
                if (Registry.GetValue(AppConfig.RegPath, key.Name, null) == null)
                {
                    Regediter.Regedit(key.Name, key.Value);
                }
            }

            #endregion 基础路径

            #region Song路径

            JObject songPath = new();
            songPath.Add("SongPostion", "");
            songPath.Add("SongName", "");
            songPath.Add("SongArtists", "");
            songPath.Add("SongCover", "");
            songPath.Add("SongPath", "");
            songPath.Add("SongAlbumUrl", "");
            songPath.Add("SongLrc", "");
            properties = songPath.Properties();
            foreach (JProperty key in properties)
            {
                if (RegGeter.RegGet("Song", key.Name) == null)
                {
                    Regediter.Regedit("Song", key.Name, key.Value);
                }
            }

            #endregion Song路径

            #region 账号路径

            JObject accountPath = new();
            accountPath.Add("Token", "");
            accountPath.Add("TokenMD5", "");
            accountPath.Add("AccountName", "");
            accountPath.Add("AccountFace", "");
            properties = accountPath.Properties();
            foreach (JProperty key in properties)
            {
                if (RegGeter.RegGet("Account", key.Name) == null)
                {
                    Regediter.Regedit("Account", key.Name, key.Value);
                }
            }

            #endregion 账号路径

            Res.res.serverPort = Registry.GetValue(AppConfig.RegPath, "ServerPort", Res.res.serverPort).ToString();
            Res.res.CVolume = int.Parse(Registry.GetValue(AppConfig.RegPath, "CurrentVolume", "100").ToString());
            MusicPlayer.InitPlayer();
            switch (RegGeter.RegGet("Theme").ToString())
            {
                case "Light":
                    Res.res.CurrentTheme = ThemeType.Light;
                    break;

                case "Dark":
                    Res.res.CurrentTheme = ThemeType.Dark;
                    break;
                default:
                    Res.res.CurrentTheme = ThemeType.Dark;
                    break;
            }

        }
    }
}
