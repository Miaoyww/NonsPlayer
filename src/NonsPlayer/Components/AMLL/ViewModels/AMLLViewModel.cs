using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using NonsPlayer.Components.AMLL.Models;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.AMLL.ViewModels;

public partial class AMLLViewModel : ObservableRecipient
{
    public ObservableCollection<LyricCombiner> LyricItems = new();
    [ObservableProperty] private IMusic currentMusic;

    #region 命令

    [RelayCommand]
    public void SwitchPlayMode()
    {
    }

    [RelayCommand]
    public void SwitchShuffle()
    {
    }

    #endregion

    public AMLLViewModel()
    {
        // LyricPositionGetter += OnLyricPositionGetter;
        Player.Instance.MusicChanged += OnMusicChanged;
        if (MusicStateModel.Instance.CurrentMusic != null)
        {
            OnMusicChanged(MusicStateModel.Instance.CurrentMusic);
        }
    }

    private async void OnMusicChanged(IMusic value)
    {
        LyricItems.Clear();

        if (value.Lyric == null)
        {
            LyricItems.Add(new LyricCombiner
            {
                LyricItemModel = new LyricItemModel(new LyricLine("暂无歌词")),
                Index = 0
            });
            return;
        }

        if (value.Lyric.Count == 0)
        {
            LyricItems.Add(new LyricCombiner
            {
                LyricItemModel = new LyricItemModel(new LyricLine("纯音乐,请欣赏")),
                Index = 0
            });
        }

        try
        {
            for (int i = 0; i < value.Lyric.Count; i++)
            {
                LyricItems.Add(new LyricCombiner
                {
                    LyricItemModel = new LyricItemModel(value.Lyric.LyricLines[i]),
                    Index = i
                });
            }
        }
        catch (Exception e)
        {
            Console.WriteLine(e);
        }
    }
}