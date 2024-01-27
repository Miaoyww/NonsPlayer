using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Models;

public class Artist : INonsModel
{
    public string Description;
    public List<Music> HotMusics;
    public int MusicCount;
    public int MvCount;
    public string Trans;
    public string MiddleAvatarUrl => AvatarUrl + "?param=500y500";
    public string SmallAvaterUrl => AvatarUrl + "?param=100y100";

    public static Artist CreatEmpty()
    {
        return new Artist
        {
            Name = "未知艺术家"
        };
    }
}