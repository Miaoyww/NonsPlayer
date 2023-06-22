using Microsoft.UI.Dispatching;
using NonsPlayer.Framework.Player;

namespace NonsPlayer.Helpers;

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