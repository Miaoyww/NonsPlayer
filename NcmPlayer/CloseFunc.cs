using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System.Text;
using System;
using System.Xml.Linq;
using System.IO;

namespace NcmPlayer
{
    public static class OnCloseFunc
    {
        public static void Close()
        {
            Regediter.Regedit("Song", "SongPostion", ResEntry.songInfo.Postion);
            Regediter.Regedit("Song", "SongPath", ResEntry.songInfo.FilePath);
            Regediter.Regedit("Song", "SongAlbumUrl", ResEntry.songInfo.AlbumCoverUrl);
            Regediter.Regedit("Song", "SongDurationTime", ResEntry.songInfo.DurationTime);
            Regediter.Regedit("Song", "SongVolume", ResEntry.songInfo.Volume);
            Regediter.Regedit("Song", "SongLrc", Convert.ToBase64String(Encoding.UTF8.GetBytes(ResEntry.songInfo.LrcString)));
            Regediter.Regedit("Song", "SongName", ResEntry.songInfo.Name);
            Regediter.Regedit("Song", "SongArtists", ResEntry.songInfo.Artists);
        }
    }
}