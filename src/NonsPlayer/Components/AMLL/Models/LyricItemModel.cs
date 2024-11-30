using NonsPlayer.Core.AMLL.Models;

namespace NonsPlayer.Components.AMLL.Models;

/// <summary>
/// 一个歌词项(行)的数据模型
/// </summary>
public class LyricItemModel
{
    public LyricLine Lyric;
    private bool isShow;
    
    public LyricItemModel(LyricLine pure)
    {
        Lyric = pure;
    }
}
public class LyricCombiner
{
   public LyricItemModel LyricItemModel { get; set; }
   public int Index { get; set; }
}
