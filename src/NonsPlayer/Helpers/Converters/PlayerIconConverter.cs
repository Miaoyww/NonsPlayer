using Microsoft.UI.Xaml.Data;

namespace NonsPlayer.Helpers.Converters;

public class PlayerIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool)value)
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