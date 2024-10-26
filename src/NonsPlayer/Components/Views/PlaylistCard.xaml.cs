﻿using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;
using Windows.UI.Core;

namespace NonsPlayer.Components.Views;

public sealed partial class PlaylistCard : UserControl
{
    public PlaylistCard()
    {
        ViewModel = App.GetService<PlaylistCardViewModel>();
        InitializeComponent();

        this.ProtectedCursor = InputCursor.CreateFromCoreCursor(new CoreCursor(CoreCursorType.Hand, 0));
    }

    public PlaylistCardViewModel ViewModel { get; }

    public IPlaylist Playlist
    {
        set => ViewModel.Init(value);
    }

}