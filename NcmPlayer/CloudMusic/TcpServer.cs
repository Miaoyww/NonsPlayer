namespace NcmPlayer.CloudMusic
{
    /*
    public static class TcpServer
    {
        public static Socket socketListen;
        public static Socket serverSocket;

        public static void ProcessCommand(string command)
        {
            switch (command)
            {
                case "connect":
                    break;

                case "reget_info":
                    break;

                case "reset":
                    break;

                case "like":
                    break;

                case "last":
                    break;

                case "pause":
                    break;

                case "next":
                    break;
            }
        }

        public static void ServerLoad(int port)
        {
            serverSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
            // IPAddress ip = IPAddress.Parse("112.248.26.221");
            IPEndPoint ip = new IPEndPoint(IPAddress.Any, 4000);
            EndPoint point = ip;
            serverSocket.Bind(point);
            serverSocket.Listen(0);
            Thread listen = new Thread(ListenClientSocket);
            listen.Start();
            PostInfo.InputLog("服务启动成功, 开放端口: ", port.ToString());
        }

        public static void ListenClientSocket()
        {
            while (true)
            {
                Socket clientSocket = serverSocket.Accept();//接受客户端的连接
                byte[] buffer = new byte[1024 * 1024];
                int num;
                try
                {
                    num = clientSocket.Receive(buffer);
                    string str = Encoding.UTF8.GetString(buffer, 0, num);
                    if (str != null)
                    {
                        JObject data = (JObject)JsonConvert.DeserializeObject(str);
                        if (data != null)
                        {
                            ProcessCommand(data["command"].ToString());
                            if (!data["command"].ToString().Equals("reget_info"))
                            {
                                PostInfo.InputLog($"收到{data["name"]}的命令: {data["command"]}");
                            }
                            CloudMusic.GetCover();
                            byte[] imageData = CloudMusic.ReadFile(CloudMusic.copiedPath);
                            string imgB64Data = Convert.ToBase64String(imageData);
                            JObject jsonData = new JObject();
                            jsonData.Add("title", CloudMusic.title);
                            jsonData.Add("artist", CloudMusic.artist);
                            jsonData.Add("albumPic", imgB64Data);
                            try
                            {
                                clientSocket.Send(Encoding.UTF8.GetBytes(jsonData.ToString()));
                                clientSocket.Close();
                            }
                            catch (SocketException)
                            {
                            }
                        }
                    }
                }
                catch (SocketException)
                {
                }
            }
        }
    }*/
}