using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.AMLL.Components.ViewModels;
using NonsPlayer.Components.AMLL.Models;

namespace NonsPlayer.Components.AMLL.Components.Views;

public sealed partial class LyricCard : UserControl
{
    public LyricCardViewModel ViewModel
    {
        get;
    }

    public LyricCard()
    {
        ViewModel = App.GetService<LyricCardViewModel>();
        InitializeComponent();
    }
    
    
    public LyricItemModel Lyric
    {
        set => ViewModel.LyricModel = value;
    }
    public int Index
    {
        set => ViewModel.Index = value;
    }
}