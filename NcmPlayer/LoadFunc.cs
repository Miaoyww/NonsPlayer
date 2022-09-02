using Microsoft.Win32;
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
            basePath.Add("ServerPort", ResEntry.res.serverPort);
            basePath.Add("DailySignin", ResEntry.res.allowDailySignin);
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
            songPath.Add("SongDurationTime", "");
            songPath.Add("SongName", "");
            songPath.Add("SongArtists", "");
            songPath.Add("SongVolume", 50);
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
            accountPath.Add("AccountFaceUrl", "");
            accountPath.Add("AccountId", "");
            accountPath.Add("LastSignin", "0");
            accountPath.Add("Likelist", "");
            properties = accountPath.Properties();
            foreach (JProperty key in properties)
            {
                if (RegGeter.RegGet("Account", key.Name) == null)
                {
                    Regediter.Regedit("Account", key.Name, key.Value);
                }
            }

            #endregion 账号路径

            ResEntry.res.serverPort = RegGeter.RegGet("ServerPort").ToString() ?? ResEntry.res.serverPort;
            switch (RegGeter.RegGet("Theme").ToString())
            {
                case "Light":
                    ResEntry.res.CurrentTheme = ThemeType.Light;
                    break;

                case "Dark":
                    ResEntry.res.CurrentTheme = ThemeType.Dark;
                    break;

                default:
                    ResEntry.res.CurrentTheme = ThemeType.Dark;
                    break;
            }
        }
    }
}