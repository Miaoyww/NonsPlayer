namespace NonsPlayer.Core.Enums;

public enum MusicQualityLevel
{
    Standard,
    Higher,
    ExHigh,
    Lossless,
    Hires,
    Effect,
    Sky,
    Master
}

public static class MusicQualityLevelConvert
{
    public static string ToQualityString(MusicQualityLevel level)
    {
        return level switch
        {
            MusicQualityLevel.Standard => "standard",
            MusicQualityLevel.Higher => "higher",
            MusicQualityLevel.ExHigh => "exhigh",
            MusicQualityLevel.Lossless => "lossless",
            MusicQualityLevel.Hires => "hires",
            MusicQualityLevel.Effect => "jyeffect",
            MusicQualityLevel.Sky => "sky",
            MusicQualityLevel.Master => "jymaster",
            _ => throw new ArgumentOutOfRangeException(nameof(level), level, null)
        };
    }
}