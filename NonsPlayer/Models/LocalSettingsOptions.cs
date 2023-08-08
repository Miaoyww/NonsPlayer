namespace NonsPlayer.Heplers;

public class LocalSettingsOptions
{
    public string? ApplicationDataFolder { get; set; }

    public string? LocalSettingsFile { get; set; }
    
    /// <summary>
    /// 播放歌单内音乐直接将歌曲添加至播放列表中
    /// </summary>
    public bool IsPlay2List { get; set; }

}