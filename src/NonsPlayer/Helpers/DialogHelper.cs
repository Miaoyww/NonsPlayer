using Microsoft.UI.Xaml.Controls;
using static NonsPlayer.Core.Services.ExceptionService;

namespace NonsPlayer.Helpers;

public class DialogHelper
{
    public static DialogHelper Instance { get; } = new();
    public delegate void DialogShowHandle(string content);
    public event DialogShowHandle DialogShowing;


    public void Show(string content)
    {
        DialogShowing?.Invoke(content);
    }
}