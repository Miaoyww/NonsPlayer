// using System.Text.Json.Serialization;
// using System.Text.RegularExpressions;
// using LyricParser.Abstraction;
// using Newtonsoft.Json.Linq;
//
// namespace NonsPlayer.Core.Models;
//
// public class LyricGroup
// {
//     [JsonPropertyName("lyrics")] public LrcLyricCollection Lyrics;
//     [JsonPropertyName("trans_lyrics")] public LrcLyricCollection? TransLyrics;
//     
//     public LyricGroup(LrcLyricCollection originalLyric, LrcLyricCollection transLyric = null)
//     {
//         Lyrics = originalLyric;
//         TransLyrics = transLyric;
//     }
//
// }
//
// public class Lyric
// {
//     [JsonPropertyName("original_lyric")] public string OriginalLyric;
//
//     [JsonPropertyName("time")] public TimeSpan Time;
//
//     [JsonPropertyName("tran_lyric")] public string TranLyric;
//
//     public Lyric(string oLrc, string tLyric, TimeSpan time)
//     {
//         OriginalLyric = oLrc;
//         TranLyric = tLyric;
//         Time = time;
//     }
// }