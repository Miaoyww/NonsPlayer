using NonsPlayer.AMLL.Models;
using NonsPlayer.Core.AMLL.Models;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using static NonsPlayer.Core.Nons.Player.Player;

namespace NonsPlayer.AMLL.Helpers;

public class LyricHelper
{
    public static LyricHelper Instance { get; } = new();

    public int LyricPosition;

    public delegate void LyricChangedHandler(LyricLine time);

    public LyricChangedHandler? LyricChanged;
}