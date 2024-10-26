using System.Diagnostics;

namespace NonsPlayer.Core.Services;

public class ExceptionService
{
    public delegate void ExceptionThrewHandle(string content);

    public static ExceptionService Instance { get; } = new();
    
    public event ExceptionThrewHandle ExceptionThrew;

    public void Throw(string content)
    {
        Debug.WriteLine($"抛出了一个异常: {content}");
        ExceptionThrew?.Invoke(content);
    }

    public void Throw(Exception exception)
    {
        Debug.WriteLine($"抛出了一个异常: {exception}");
        ExceptionThrew?.Invoke(exception.Message);
    }

    public void Throw(Exception exception, string content)
    {
        Debug.WriteLine($"抛出了一个异常: {exception}");
        ExceptionThrew?.Invoke(content);
    }
}