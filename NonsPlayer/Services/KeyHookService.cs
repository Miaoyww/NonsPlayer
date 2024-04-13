using System.Diagnostics;
using System.Windows.Forms;
using Windows.Win32;
using Windows.Win32.Foundation;
using Windows.Win32.UI.Input.KeyboardAndMouse;
using Gma.System.MouseKeyHook;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Services;

public class KeyHookService
{
    private IntPtr _hwnd;

    public static KeyHookService Instance { get; } = new();

    public void Init()
    {
        var volumeUp = Combination.TriggeredBy(Keys.Up).With(Keys.Control);
        var volumeDown = Combination.TriggeredBy(Keys.Down).With(Keys.Control);
        var play = Combination.TriggeredBy(Keys.Oem3).With(Keys.Control).With(Keys.Alt);
        var next = Combination.TriggeredBy(Keys.Right).With(Keys.Control);
        var previous = Combination.TriggeredBy(Keys.Left).With(Keys.Control);
        var assignment = new Dictionary<Combination, Action>
        {
            { volumeUp, VolumeUp },
            { volumeDown, VolumeDown },
            { play, Play },
            { next, Next },
            { previous, Previous }
        };

        Hook.GlobalEvents().OnCombination(assignment);
    }


    private void VolumeUp()
    {
        MusicStateModel.Instance.Volume += AppConfig.VolumeAddition;
    }

    private void VolumeDown()
    {
        if (MusicStateModel.Instance.Volume -
            AppConfig.VolumeAddition <= 0)
        {
            MusicStateModel.Instance.Volume = 0;
            return;
        }

        MusicStateModel.Instance.Volume -=
            AppConfig.VolumeAddition;
    }


    private void Play()
    {
        Player.Instance.Play();
    }

    private void Previous()
    {
        PlayQueue.Instance.PlayPrevious();
    }

    private void Next()
    {
        PlayQueue.Instance.PlayNext();
    }

    private void Like()
    {
        // TODO
    }
}