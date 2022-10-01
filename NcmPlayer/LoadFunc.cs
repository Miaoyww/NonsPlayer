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

            #region Music路径

            JObject musicPath = new();
            musicPath.Add("MusicPostion", "");
            musicPath.Add("MusicDurationTime", "");
            musicPath.Add("MusicName", "");
            musicPath.Add("MusicArtists", "");
            musicPath.Add("MusicVolume", 50);
            musicPath.Add("MusicCover", "");
            musicPath.Add("MusicPath", "");
            musicPath.Add("MusicAlbumUrl", "");
            musicPath.Add("MusicLrc", "");
            properties = musicPath.Properties();
            foreach (JProperty key in properties)
            {
                if (RegGeter.RegGet("Music", key.Name) == null)
                {
                    Regediter.Regedit("Music", key.Name, key.Value);
                }
            }

            #endregion Music路径

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