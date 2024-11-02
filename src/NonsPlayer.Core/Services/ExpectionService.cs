using System.Diagnostics;

namespace NonsPlayer.Core.Services;

public class ExceptionService
{
    public delegate void ExceptionThrewHandle(string content);

    public static ExceptionService Instance { get; } = new();
    
    public event ExceptionThrewHandle ExceptionThrew;

    public void Throw(string content)
    {
        Debug.WriteLine($"Threw an Exception: {content}");
        ExceptionThrew?.Invoke(content);
    }

    public void Throw(Exception exception)
    {
        Debug.WriteLine($"Threw an Exception:: {exception}");
        ExceptionThrew?.Invoke(exception.Message);
    }

    public void Throw(Exception exception, string content)
    {
        Debug.WriteLine($"Threw an Exception:: {exception}");
        ExceptionThrew?.Invoke(content);
    }
}