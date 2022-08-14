using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

namespace NeteaseCloudMusicControl.Services
{
    public static class TcpServer
    {
        public static Socket socketListen;
        public static Socket serverSocket;

        public static void Listener()
        {
            while (true)
            {
                try
                {
                    serverSocket = socketListen.Accept();
                }
                catch (InvalidOperationException)
                {
                    continue;
                }
                while (true)
                {
                    try
                    {
                        byte[] buffer = new byte[1024 * 1024];
                        int num = serverSocket.Receive(buffer);
                        string str = Encoding.UTF8.GetString(buffer, 0, num);
                        JObject data = (JObject)JsonConvert.DeserializeObject(str);
                        if (data != null)
                        {
                            PostInfo.InputLog($"收到{data["name"]}的命令: {data["command"]}");
                            serverSocket.Send(Encoding.UTF8.GetBytes("服务端收到数据包"));
                            ProcessCommand(data["command"].ToString());
                            break;
                        }
                        else
                        {
                            break;
                        }

                    }
                    catch (NullReferenceException)
                    {
                        break;
                    }
                    catch (SocketException)
                    {
                        break;
                    }
                }
            }
        }

        public static void ProcessCommand(string command)
        {
            switch (command)
            {
                case "reset":
                    CloudMusic.ResetWindow();
                    break;
                case "like":
                    CloudMusic.LikeSound();
                    break;
                case "last":
                    CloudMusic.LastSound();
                    break;
                case "pause":
                    CloudMusic.PauseSound();
                    break;
                case "next":
                    CloudMusic.NextSound();
                    break;
            }
        }

        public static void ServerLoad(int port)
        {
            
            IPAddress ips = IPAddress.Parse("127.0.0.1");
            IPEndPoint ipNode = new IPEndPoint(ips, port);
            socketListen = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
            try
            {
                socketListen.Bind(ipNode);
                socketListen.Listen(10);

                PostInfo.InputLog("服务启动成功, 开放端口: ", port.ToString());
                Thread listenerThread = new Thread(() =>
                {
                    Listener();
                });
                listenerThread.Start();
                listenerThread.IsBackground = true;
            }
            catch (SocketException)
            {
                PostInfo.InputLog("端口已被占用");
            }
        }
    }
}
