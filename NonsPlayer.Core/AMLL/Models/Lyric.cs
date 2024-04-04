namespace NonsPlayer.Core.AMLL.Models;

public class Lyric
{
    // 用于储存一首歌的所有歌词

    public List<LyricLine> LyricLines { get; set; }
    public int Count => LyricLines.Count;
    public Lyric(List<LyricLine> lyricLines)
    {
        LyricLines = lyricLines;
    }
    
    /// <summary>
    /// 添加歌词原文。若歌词为逐字歌词，需要翻译时必须调用此方法
    /// </summary>
    /// <param name="pure"></param>
    public void CombinePure(Lyric pure)
    {
        for (int i = 0; i < LyricLines.Count - 1; i++)
        {
            LyricLines[i].Pure = pure.LyricLines[i].Pure;
            // 保留原始时间，与翻译歌词的时间对应
            LyricLines[i].StartTime = pure.LyricLines[i].StartTime;
            LyricLines[i].EndTime = pure.LyricLines[i].EndTime;
        }
    }
    public void AddTrans(Lyric? translatedLyrics)
    {
        if (translatedLyrics == null || translatedLyrics.LyricLines.Count == 0)
        {
            return;
        }

        int translatedIndex = 0; // 翻译行索引
        for (int i = 0; i < LyricLines.Count; i++)
        {
            var originalLine = LyricLines[i];
            var translatedLine = translatedLyrics.LyricLines[translatedIndex];
            if (originalLine.StartTime != translatedLine.StartTime)
            {
                translatedLine = new LyricLine();
            }
            else
            {
                translatedIndex++;
            }

            originalLine.SetTranslation(translatedLine.Pure);
        }
    }
}