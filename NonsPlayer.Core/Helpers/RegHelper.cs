using Microsoft.Win32;

namespace NonsPlayer.Core.Helpers;

public class RegHelper
{
    public static RegHelper Instance { get; } = new();

    // 程序的注册表路径为：HKEY_CURRENT_USER\Software\NonsPlayer
    public RegistryKey RegKey { get; } = Registry.CurrentUser.CreateSubKey(@"Software\NonsPlayer");

    // 如果项存在，返回true，否则返回false
    private bool CheckItem(string name)
    {
        if (RegKey.GetValue(name, "") == "")
        {
            RegKey.SetValue(name, "");
            return false;
        }

        return true;
    }

    public void Set(string name, object value)
    {
        CheckItem(name);
        if (value != null) RegKey.SetValue(name, value);
    }


    public object? Get(string name)
    {
        CheckItem(name);

        return RegKey.GetValue(name, null);
    }

    public object? Get(string name, object defaultValue)
    {
        return !CheckItem(name) ? defaultValue : RegKey.GetValue(name, defaultValue);
    }

    public static class Regs
    {
        public static string Position { get; } = "Position";

        public static string Volume { get; } = "Volume";

        public static string AccountToken { get; } = "AccountToken";

        public static string AccountTokenMd5 { get; } = "AccountTokenMd5";


        public static string Mute { get; } = "Mute";
    }
}