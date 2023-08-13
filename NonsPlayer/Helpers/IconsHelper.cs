using Microsoft.UI.Xaml.Data;
using NonsPlayer.Core.Enums;

namespace NonsPlayer.Helpers;

public class PlayerIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool) value)
            // 正在播放
            return "\uf8ae";
        // 未播放
        return "\uf5b0";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return value.ToString() == "\uf8ae";
    }
}

public class VolumeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return (double) value switch
        {
            var n when n <= 10 => "\ue992",
            var n when n is <= 40 and > 10 => "\ue993",
            var n when n is <= 80 and > 40 => "\ue994",
            var n when n is <= 100 and > 80 => "\ue995",
            _ => "\ue992"
        };
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}

public class LikeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool) value) return "\uEB52";

        return "\uEB51";
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}

public class SearchDataTypeToString : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var item = value as SearchDataType?;
        switch (item)
        {
            case SearchDataType.Music:
                return "单曲";
            case SearchDataType.Artist:
                return "艺术家";
            case SearchDataType.Playlist:
                return "歌单";
            case SearchDataType.Album:
                return "专辑";
            default:
                return "未知";
        }
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return "未知";
    }
}