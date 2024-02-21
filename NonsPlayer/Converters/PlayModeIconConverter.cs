using System.Drawing;
using System.Numerics;
using System.Runtime.InteropServices;
using Microsoft.UI;
using Microsoft.UI.Composition;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Nons.Player;
using WinRT.Interop;
using Color = Windows.UI.Color;

namespace NonsPlayer.Converters;

public class PlayModeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var mode = (PlayQueue.PlayModeEnum)value;
        switch (mode)
        {
            case PlayQueue.PlayModeEnum.Sequential:
                return "\uf5e7";
            case PlayQueue.PlayModeEnum.Recommend:
                return "\ue704";
            case PlayQueue.PlayModeEnum.SingleLoop:
                return "\ue8ed";
            case PlayQueue.PlayModeEnum.ListLoop:
                return "\ue8ee";
            default:
                return "\ue8ee";
        }
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}