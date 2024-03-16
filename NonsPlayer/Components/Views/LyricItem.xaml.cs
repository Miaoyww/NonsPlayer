using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LyricParser.Abstraction;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.Services;
using NonsPlayer.ViewModels;
using UserControl = Microsoft.UI.Xaml.Controls.UserControl;

namespace NonsPlayer.Components.Views;

public sealed partial class LyricItem : UserControl
{
    public LyricItem()
    {
        ViewModel = App.GetService<LyricItemViewModel>();
        InitializeComponent();
    }

    public LyricItemViewModel ViewModel { get; }

    public SongLyric Lyric
    {
        set => ViewModel.SongLyric = value;
    }
    public int Index
    {
        set => ViewModel.Index = value;
    }
    public Thickness Margin
    {
        set => ViewModel.Margin = value;
    }
}