using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class LocalMusic : IMusic
{
    /*本地音乐
    包含音乐文件的路径、音乐文件的元数据、音乐文件的歌词数据

     */
    public byte[]? Cover;
    public TagLib.File File;
    public string CacheId => Md5 + "_music";
    public LocalMusic(string path)
    {
        File = TagLib.File.Create(path);

        Uri = path;
        Cover = File.Tag.Pictures.Length > 0 ? File.Tag.Pictures[0].Data.Data : null;
        Md5 = File.GetHashCode().ToString();
        Album = new Album()
        {
            Name = File.Tag.Album
        };
        Artists = new Artist[]
        {
            new Artist()
            {
                Name = File.Tag.FirstPerformer
            }
        };
        Duration = File.Properties.Duration;
        Name = File.Tag.Title;
        
    }
}