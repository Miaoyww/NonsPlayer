using Microsoft.Win32;
using NeteaseCloudMusicControl.Config;
using NeteaseCloudMusicControl.Services;

namespace NeteaseCloudMusicControl
{
    class Program
    {
        static void Main(string[] args)
        {
            CloudMusic.Reload();
            CloudMusic.timer.Elapsed += new System.Timers.ElapsedEventHandler(CloudMusic.GetSoundInfo);
            CloudMusic.timer.Interval = 100;
            CloudMusic.timer.Start();
            /*
            object? keyvalue = Registry.GetValue(AppConfig.RegPath, "Port", null);
            if (keyvalue is null)
            {
                Registry.SetValue(AppConfig.RegPath, "Port", 4000);
            }
            int port = (int)Registry.GetValue(AppConfig.RegPath, "Port", 4000);
            */
            int port = 4000;
            Thread tcpListener = new(() =>
            {
                TcpServer.ServerLoad(port);
            });
            tcpListener.Start();
            tcpListener.IsBackground = true;
            Console.ReadKey();
        }
        public static void Print(string msg)
        {
            Console.WriteLine(msg);
        }
    }
}