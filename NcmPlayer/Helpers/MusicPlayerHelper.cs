using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Dispatching;
using NcmPlayer.Framework.Player;

namespace NcmPlayer.Helpers;

public static class MusicPlayerHelper
{
    public static MusicPlayer Player;

    public static async void InitPlayer(DispatcherQueue dispatcher)
    {
        Player = new MusicPlayer()
        {
            Volume = 0,
            IsPlaying = false,
            Position = default
        };
        Player.InitPlayer(dispatcher);
    }
}