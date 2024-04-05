using System.Text.Json.Serialization;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class LocalMusic : IMusic
{
    /*本地音乐
    包含音乐文件的路径、音乐文件的元数据、音乐文件的歌词数据

     */
    [JsonIgnore] public TagLib.File File;

    public LocalMusic(string path)
    {
        File = TagLib.File.Create(path);
        LocalCover = File.Tag.Pictures.Length > 0 ? File.Tag.Pictures[0].Data.Data : null;
        Md5 = File.GetHashCode().ToString();
        Uri = path;
        Album = new Album()
        {
            Name = File.Tag.Album,
            Id = $"{File.Tag.Album}_{Md5}".GetHashCode(),
            AvatarUrl = Uri,
        };
        Artists =
        [
            new()
            {
                Name = File.Tag.FirstPerformer,
                Id = $"{File.Tag.FirstPerformer}_{Md5}".GetHashCode()
            }
        ];
        Duration = File.Properties.Duration;
        Name = File.Tag.Title;
        // Id = $"{Name}_{Md5}".GetHashCode();
    }
}