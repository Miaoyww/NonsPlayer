﻿using Microsoft.UI.Xaml.Data;
using NonsPlayer.Core.Nons.Player;

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