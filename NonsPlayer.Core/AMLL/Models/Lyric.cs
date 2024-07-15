using System.Diagnostics;

namespace NonsPlayer.Core.AMLL.Models;


/// <summary>
/// 一首歌的所有歌词
/// </summary>
public class Lyric
{
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
        if (translatedLyrics.Count == LyricLines.Count)
        {
            // 逐行翻译
            for (int i = 0; i < LyricLines.Count; i++)
            {
                var originalLine = LyricLines[i];
                var translatedLine = translatedLyrics.LyricLines[i];

                originalLine.SetTranslation(translatedLine.Pure);
            }
        }

        for (int i = 0; i < LyricLines.Count; i++)
        {
            var originalLine = LyricLines[i];
            LyricLine translatedLine;
            try
            {
                translatedLine = translatedLyrics.LyricLines[translatedIndex];
            }
            catch (ArgumentException e)
            {
                translatedLine = new LyricLine();
                Debug.WriteLine($"抛出了一个异常: 原歌词数{LyricLines.Count},翻译歌词数{translatedLyrics.Count}\n" + e);
            }

            if (originalLine.StartTime != translatedLine.StartTime)
            {
                translatedLine = new LyricLine();
            }
            else
            {
                translatedIndex++;
            }

            Debug.WriteLine(
                $"{i} - {translatedIndex}: {originalLine.Pure}, {translatedLine.Pure} - {originalLine.StartTime == translatedLine.StartTime}");
            originalLine.SetTranslation(translatedLine.Pure);
        }
    }
}