using Microsoft.UI.Xaml;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage.Streams;

namespace NonsPlayer;

public sealed partial class CrashWindow : WindowEx
{
    private Exception exception;
    public CrashWindow(Exception ex)
    {
        InitializeComponent();
        exception = ex;
        ExceptionText.Text = ex.ToString();
    }

    private void ButtonBase_OnClick(object sender, RoutedEventArgs e)
    {
        var data = new DataPackage();
        data.SetText(ExceptionText.Text);
        Clipboard.SetContent(data);
    }
}