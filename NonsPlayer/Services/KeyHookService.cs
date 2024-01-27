using System.Diagnostics;
using System.Windows.Forms;
using Windows.Win32;
using Windows.Win32.Foundation;
using Windows.Win32.UI.Input.KeyboardAndMouse;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Services;

public class KeyHookService
{
    private IntPtr _hwnd;

    public static KeyHookService Instance { get; } = new();

    public void Init(IntPtr hwnd)
    {
        _hwnd = hwnd;
        RegKey(1, HOT_KEY_MODIFIERS.MOD_CONTROL | HOT_KEY_MODIFIERS.MOD_ALT, Keys.Oem3); // 播放/暂停
        RegKey(2, HOT_KEY_MODIFIERS.MOD_CONTROL, Keys.Right); // 下一首
        RegKey(3, HOT_KEY_MODIFIERS.MOD_CONTROL, Keys.Left); // 上一首
        RegKey(4, HOT_KEY_MODIFIERS.MOD_CONTROL, Keys.Up); // 音量+
        RegKey(5, HOT_KEY_MODIFIERS.MOD_CONTROL, Keys.Down); // 音量-
    }

    private void RegKey(int id, HOT_KEY_MODIFIERS modifiers, Keys keys)
    {
        UnRegKey(id);
        if (PInvoke.RegisterHotKey((HWND)_hwnd, id, modifiers, (uint)keys))
            Debug.WriteLine($"注册成功{modifiers}+{keys}");
        else
            Debug.WriteLine("注册失败");
    }

    private void UnRegKey(int id)
    {
        PInvoke.UnregisterHotKey((HWND)_hwnd, id);
    }

    /// <summary>
    ///     1:播放/暂停 2:下一首 3:上一首 4:音量+ 5:音量- 6:喜欢歌曲
    /// </summary>
    /// <param name="id"></param>
    public void OnHotKey(int id)
    {
        switch (id)
        {
            case 0x1:
                Debug.WriteLine("按下了Ctrl+Alt+~");
                Player.Instance.Play();
                break;
            case 0x2:
                Debug.WriteLine("按下了Ctrl+→");
                PlayQueue.Instance.PlayNext();
                break;
            case 0x3:
                Debug.WriteLine("按下了Ctrl+←");
                PlayQueue.Instance.PlayPrevious();
                break;
            case 0x4:
                Debug.WriteLine("按下了Ctrl+↑");
                MusicStateModel.Instance.Volume += App.GetService<ILocalSettingsService>().GetOptions().VolumeAddition;
                break;
            case 0x5:
                Debug.WriteLine("按下了Ctrl+↓");
                if (MusicStateModel.Instance.Volume -
                    App.GetService<ILocalSettingsService>().GetOptions().VolumeAddition <= 0)
                {
                    MusicStateModel.Instance.Volume = 0;
                    break;
                }

                MusicStateModel.Instance.Volume -=
                    App.GetService<ILocalSettingsService>().GetOptions().VolumeAddition;
                break;
        }
    }
}