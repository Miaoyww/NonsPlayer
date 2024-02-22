using System.Diagnostics;

namespace NonsPlayer.Core.Services;

public class ExceptionService
{
    public delegate void ExceptionThrewHandle(Exception exception);


    public static ExceptionService Instance { get; } = new();

    public event ExceptionThrewHandle ExceptionThrew;

    public void Throw(Exception exception)
    {
        Debug.WriteLine($"抛出了一个异常: {exception}");
        ExceptionThrew?.Invoke(exception);
    }
}