using System.Diagnostics;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;


namespace NonsPlayer.Core.Services;

public class ExceptionService
{
    public delegate void ExceptionThrewHandle(Exception exception);

    public event ExceptionThrewHandle ExceptionThrew;


    public static ExceptionService Instance { get; } = new();

    public void Throw(Exception exception)
    {
        Debug.WriteLine($"抛出了一个异常: {exception}");
        ExceptionThrew?.Invoke(exception);
    }
}