using System;
using System.IO;
using System.Windows.Controls;

namespace NeteaseCloudMusicControl
{
    public static class CurrentResources
    {
        // 播放器
        public static MediaElement musicplayer;

        // 当前播放音乐的封面
        public static Stream currentPlayCover;

        // 当前播放音乐的名称
        public static string currentPlayName = string.Empty;

        public static string __currentPlayPostionString = string.Empty;
        public static int __currentPlayPostion;
        public static string __currentPlayWholeTimeString = string.Empty;
        public static int __currentPlayWholeTime;
        // 当前播放进度
        public static string currentPlayPostionString
        {
            set
            {
                TimeSpan timespan = TimeSpan.FromSeconds(int.Parse(value));
                int min = timespan.Minutes;
                int sec = timespan.Seconds;
                __currentPlayPostionString = $"{min}:{sec}";
                
            }
            get
            {
                if (!__currentPlayPostionString.Equals(string.Empty))
                {
                    return __currentPlayPostionString;
                }
                else
                {
                    return "0:00";
                }
            }
        }

        public static int currentPlayPostion
        {
            set
            {
                currentPlayPostionString = value.ToString();
                __currentPlayPostion = value;
            }
            get
            {
                return __currentPlayPostion;
            }
        }

        // 当前播放总时长
        public static string currentPlayWholeTimeString
        {
            set
            {
                TimeSpan timespan = TimeSpan.FromSeconds(int.Parse(value));
                int min = timespan.Minutes;
                int sec = timespan.Seconds;
                __currentPlayWholeTimeString = $"{min}:{sec}";
            }
            get
            {
                if (!__currentPlayPostionString.Equals(string.Empty))
                {
                    return __currentPlayWholeTimeString;
                }
                else
                {
                    return "0:00";
                }
            }
        }

        public static int currentPlayWholeTime
        {
            set
            {
                currentPlayWholeTimeString = value.ToString();
                __currentPlayWholeTime = value;
            }
            get
            {
                return __currentPlayWholeTime;
            }
        }

        // 当前播放音乐的音乐家们
        public static string[] currentPlayArtists;

        // 当前播放音乐的路径
        public static string soundPath = string.Empty;

        // 当前是否在播放
        public static bool isPlayed = false;

        // 当前的音量
        public static string currentVolume = string.Empty;

        public static string serverIp = "127.0.0.1";

        public static string serverPort = "4000";

        public static string log = string.Empty;

        public static string soundsPath = "./sounds.json";
    }
}