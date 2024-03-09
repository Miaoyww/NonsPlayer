using System.Diagnostics;
using System.Windows.Forms;
using CommunityToolkit.Mvvm.Input;
using LyricParser.Abstraction;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
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

    public ILyricLine? PureLyric
    {
        set => ViewModel.PureLyric = value;
    }

    public string? TransLyric
    {
        set => ViewModel.TransLyric = value;
    }    
    public Visibility TransVisibility
    {
        set => ViewModel.TransVisibility = value;
    }
}