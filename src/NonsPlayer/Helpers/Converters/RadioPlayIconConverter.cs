using Microsoft.UI.Xaml.Data;
using NonsPlayer.Services;

namespace NonsPlayer.Helpers.Converters;

public class RadioPlayIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool)value)
        {
            if (App.GetService<RadioService>().IsStarted)
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