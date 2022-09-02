using Microsoft.Win32;

namespace NcmPlayer.Resources
{
    public class Regediter
    {
        public static void Regedit(string key, object value)
        {
            Registry.SetValue(AppConfig.RegPath, key, value);
        }

        public static void Regedit(string path, string key, object value)
        {
            if(value != null)
            {
                Registry.SetValue(AppConfig.RegPath + path, key, value);
            }
        }
    }

    public class RegGeter
    {
        public static object? RegGet(string key)
        {
            return Registry.GetValue(AppConfig.RegPath, key, null);
        }

        public static object? RegGet(string path, string key)
        {
            return Registry.GetValue(AppConfig.RegPath + path, key, null);
        }
    }
}