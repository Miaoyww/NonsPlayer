using Microsoft.UI.Xaml.Data;

namespace NonsPlayer.Helpers;

public class PlayerIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool)value)
        {
            // 正在播放
            return "\uf8ae";
        }
        else
        {
            // 未播放
            return "\uf5b0";
        }
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
        return (double)value switch
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