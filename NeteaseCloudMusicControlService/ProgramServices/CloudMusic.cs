using NeteaseCloudMusicControl.Config;
using System.Runtime.InteropServices;
using System.Timers;
using System.Threading;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json;
using static System.Net.Mime.MediaTypeNames;
using System.IO;
using System.Net;
using System.Text;

namespace NeteaseCloudMusicControl.Services
{
    public static class CloudMusic
    {
        public static System.Timers.Timer timer = new System.Timers.Timer();
        public static string title;
        public static string artist;
        public static string albumId;
        public static string albumPicUrl;
        public static string albumPicPath;
        public static string postMes = "已提交指令:";
        public static int pid;
        public static IntPtr hwnd;
        public static IntPtr maindHwnd;

        public static string tempPath = $"{Path.GetTempPath()}alpics\\";

        [DllImport("user32.dll")]
        public static extern bool PostMessage(IntPtr hWnd, uint Msg, int wParam, int lParam);

        [DllImport("User32.dll", EntryPoint = "SendMessage")]
        public static extern int SendMessage(IntPtr hWnd, uint Msg, int wParam, int lParam);

        [DllImport("user32.dll", EntryPoint = "FindWindow")]
        public static extern IntPtr FindWindow(string lpClassName, string? lpWindowName);

        [DllImport("user32.dll", EntryPoint = "FindWindowEx", SetLastError = true)]
        public static extern IntPtr FindWindowEx(IntPtr hwndParent, IntPtr hwndChildAfter, string? lpszClass, string? lpszWindow);

        [DllImport("user32.dll", SetLastError = true)]
        internal static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int nWidth, int nHeight, bool bRepaint);

        public static void Post(int IParam)
        {
            PostMessage(hwnd, 0x0201, 0x0001, IParam);
            PostMessage(hwnd, 0x0202, 0x0001, IParam);
        }

        public static void LikeSound()
        {
            Post(31392059);
            PostInfo.InputLog(postMes, "喜欢 \\ 取消喜欢音乐");
        }

        public static void LastSound()
        {
            Post(36372927);
            PostInfo.InputLog(postMes, "上一首");
        }

        public static void PauseSound()
        {
            Post(36176372);
            PostInfo.InputLog(postMes, "暂停 \\ 开始");
        }

        public static void NextSound()
        {
            Post(36504103);
            PostInfo.InputLog(postMes, "下一首");
        }

        public static void ResetWindow()
        {
            MoveWindow(maindHwnd, 0, 0, 1000, 600, true);
            PostInfo.InputLog("重设窗口");
        }

        public static void GetSoundInfo(object source, ElapsedEventArgs e)
        {
            string apptitle;
            User32.GetWindowTitle("OrpheusBrowserHost", out apptitle);
            if (pid == 0)
            {
                return;
            }
            else
            {
                string[] titleSpilt = apptitle.Split('-');
                if (titleSpilt.Length <= 1)
                {
                    title = AppConfig.NullTitle;
                    artist = AppConfig.NullArtists;
                }
                else
                {
                    string lastTitle = title;
                    string lastArtist = artist;
                    title = titleSpilt[0].Remove(titleSpilt[0].Length - 1, 1);
                    artist = titleSpilt[1].Remove(0, 1);
                    if (!lastTitle.Equals(title) || !lastArtist.Equals(artist))
                    {
                        PostInfo.InputLog("当前播放: ", apptitle);
                    }
                }
            }
        }

        public static void Reload()
        {
            User32.GetWindowTitle("OrpheusBrowserHost", out title, out pid);
            maindHwnd = FindWindow("OrpheusBrowserHost", null);
            if(maindHwnd != (IntPtr)0)
            {
                IntPtr childHwnd = FindWindowEx(maindHwnd, IntPtr.Zero, null, null);
                while (true)
                {
                    IntPtr temp = FindWindowEx(childHwnd, IntPtr.Zero, null, null); ;
                    if (temp == IntPtr.Zero)
                    {
                        break;
                    }
                    else
                    {
                        childHwnd = temp;
                    }
                }
                hwnd = childHwnd;
                PostInfo.InputLog("获取到网易云进程");
                PostInfo.InputLog($"进程pid: {pid}");
                PostInfo.InputLog($"进程hwnd: {hwnd}");
                PostInfo.InputLog($"当前播放: {title}");
                // 防止窗口乱跑以及定位不到
                ResetWindow();
            }
        }
        public static string ReadFile(string path)
        {
            bool _isExist = false;
            if (!File.Exists(path))
            {
                FileStream createSoundsJson = new FileStream(path, FileMode.Create);
                createSoundsJson.Flush();
                createSoundsJson.Close();
                _isExist = false;
            }
            else
            {
                _isExist = true;
            }
            using (FileStream fsSource = new FileStream(path, FileMode.Open, FileAccess.Read))
            {
                byte[] bytes = new byte[fsSource.Length];
                int numBytesToRead = (int)fsSource.Length;
                int numBytesRead = 0;
                while (numBytesToRead > 0)
                {
                    // Read may return anything from 0 to numBytesToRead.
                    int n = fsSource.Read(bytes, numBytesRead, numBytesToRead);
                    // Break when the end of the file is reached.
                    if (n == 0)
                        break;
                    numBytesRead += n;
                    numBytesToRead -= n;
                }
                fsSource.Flush();
                fsSource.Close();
                if (_isExist)
                {
                    string msg = System.Text.Encoding.UTF8.GetString(bytes);
                    if (msg.Equals(""))
                    {
                        return string.Empty;
                    }
                    byte[] buffer = Encoding.UTF8.GetBytes(msg);
                    string sResult = Encoding.UTF8.GetString(buffer, 0, buffer.Length);
                    byte[] bomBuffer = new byte[] { 0xef, 0xbb, 0xbf };
                    if (buffer[0] == bomBuffer[0] && buffer[1] == bomBuffer[1] && buffer[2] == bomBuffer[2])
                    {
                        int copyLength = buffer.Length - 3;
                        byte[] dataNew = new byte[copyLength];
                        Buffer.BlockCopy(buffer, 3, dataNew, 0, copyLength);
                        sResult = System.Text.Encoding.UTF8.GetString(dataNew);
                    }
                    return sResult;
                }
                else
                {
                    return string.Empty;
                }
            }
        }

        public static void WriteFile(string path, string content)
        {
            var writeInto = Encoding.UTF8.GetBytes(content);
            int numBytesToWrite = writeInto.Length;
            using (FileStream fsNew = new FileStream(path,
                       FileMode.Create, FileAccess.Write))
            {
                fsNew.Write(writeInto, 0, numBytesToWrite);
                fsNew.Flush();
                fsNew.Close();
            }
        }

        public static Stream HttpGet(string url)
        {
            WebRequest wrGETURL;
            wrGETURL = WebRequest.Create(url);
            Stream objStream;
            while (true)
            {
                try
                {
                    objStream = wrGETURL.GetResponse().GetResponseStream();
                    return objStream;
                }
                catch (WebException)
                {
                    PostInfo.InputLog("api连接失败, 3秒后重试");
                    Thread.Sleep(3);
                }
            }
        }

        public static void GetAlumbInfo(Stream stream, string processedTitle, string processedArtists)
        {
            StreamReader objReader = new StreamReader(stream);
            JObject result = (JObject)JsonConvert.DeserializeObject(objReader.ReadLine());

            /*
             * 为了防止匹配到非正确的专辑封面
             * 将对标题和作者进行双重匹配
             */
            foreach (JToken token in result["result"]["songs"])
            {
                string targetTitle = string.Empty;
                string targetArtist = string.Empty;
                string soundId = string.Empty;

                targetTitle = token["name"].ToString().Replace(" ", "");

                if (targetTitle.Equals(processedTitle))
                {
                    JToken artists = token["ar"];
                    foreach (JToken artistItem in artists)
                    {
                        targetArtist += artistItem["name"].ToString().Replace(" ", "") + "/";
                    }
                    if (targetArtist.Remove(targetArtist.Length - 1).Equals(processedArtists))
                    {
                        soundId = token["id"].ToString();
                    }
                    if (!soundId.Equals(string.Empty))
                    {
                        albumId = token["al"]["id"].ToString();
                        albumPicUrl = token["al"]["picUrl"].ToString();
                        break;
                    }
                }
            }
        }

        public static void Download2ApplyAlbumPic(string url, string albumid)
        {
            albumPicPath = $"./alpics/{albumId}.jpg";
            if (!Directory.Exists("./alpics"))
            {
                Directory.CreateDirectory("./alpics");
            }

            if (!Directory.Exists(CloudMusic.tempPath))
            {
                Directory.CreateDirectory(CloudMusic.tempPath);
            }

            if (!File.Exists(albumPicPath))
            {
                HttpWebRequest request = WebRequest.Create(url) as HttpWebRequest;
                HttpWebResponse response = request.GetResponse() as HttpWebResponse;
                Stream responseStream = response.GetResponseStream();
                Stream stream = new FileStream(albumPicPath, FileMode.Create);
                byte[] bArr = new byte[1024];
                int size = responseStream.Read(bArr, 0, (int)bArr.Length);
                while (size > 0)
                {
                    stream.Write(bArr, 0, size);
                    size = responseStream.Read(bArr, 0, (int)bArr.Length);
                }
                stream.Close();
                responseStream.Close();
            }
            ApplyAlbumPic(albumid);

            PostInfo.InputLog("已切换封面, 歌曲名称:", CloudMusic.title);

        }

        public static void ApplyAlbumPic(string albumid)
        {
            string copiedPath = CloudMusic.tempPath + $"{albumid}.jpg";
            if (!File.Exists(copiedPath))
            {
                File.Copy(albumPicPath, copiedPath, true);
            }
        }

        public static void GetCover()
        {
            string sURL = $"http://localhost:3000/cloudsearch?keywords={CloudMusic.title} -{CloudMusic.artist}&limit=10";

            string processedTitle = CloudMusic.title.Replace(" ", "");  // 为方便对比, 将所有空格去掉
            string processedArtist = CloudMusic.artist.Replace(" ", "");
            string readResult = ReadFile(AppConfig.SoundsPath);
            string savingContent = $"{CloudMusic.title} -{CloudMusic.artist}"; // 用于储存在sounds.json followings中的成员
            JObject result;
            JArray? albumList = new();
            if (readResult == string.Empty)
            {
                JObject? currentSoundInfo = new JObject();
                GetAlumbInfo(HttpGet(sURL), processedTitle, processedArtist);
                currentSoundInfo.Add("albumId", albumId);
                currentSoundInfo.Add("followings", new JArray() { $"{CloudMusic.title} -{CloudMusic.artist}" });
                albumList.Add(currentSoundInfo);
                Download2ApplyAlbumPic(albumPicUrl, albumId);
            }
            else
            {
                JObject? readResultJObject = JObject.Parse(readResult);
                albumList = (JArray)readResultJObject["sounds"];
                bool _titleIsExist = false;
                foreach (JObject album in albumList)
                {
                    foreach (var sound in album["followings"])
                    {
                        if (sound.ToString().Equals(savingContent))
                        {
                            albumId = album["albumId"].ToString();
                            albumPicPath = $"./alpics/{albumId}.jpg";
                            if (File.Exists(albumPicPath))
                            {
                                ApplyAlbumPic(albumId);
                            }
                            else
                            {
                                GetAlumbInfo(HttpGet(sURL), processedTitle, processedArtist);
                                Download2ApplyAlbumPic(albumPicUrl, albumId);
                            }
                            _titleIsExist = true;
                        }
                    }
                }

                GetAlumbInfo(HttpGet(sURL), processedTitle, processedArtist);
                if (!_titleIsExist)
                {
                    JObject? currentSoundInfo = new JObject();
                    int index = 0;
                    foreach (JObject album in albumList)
                    {
                        if (album["albumId"].ToString().Equals(albumId))
                        {
                            JObject currentSoundAlbum = (JObject)albumList[index];
                            albumId = currentSoundAlbum["albumId"].ToString();
                            albumPicPath = $"./alpics/{albumId}.jpg";
                            JArray soundList = (JArray)currentSoundAlbum["followings"];
                            soundList.Add(savingContent);
                            albumList.RemoveAt(index);
                            albumList.Add(currentSoundAlbum);
                            _titleIsExist = true;
                            ApplyAlbumPic(albumId);
                            break;
                        }
                        index++;
                    }
                    if (!_titleIsExist)
                    {
                        currentSoundInfo.Add("albumId", albumId);
                        currentSoundInfo.Add("followings", new JArray() { savingContent });
                        albumList.Add(currentSoundInfo);
                        Download2ApplyAlbumPic(albumPicUrl, albumId);
                    }

                }
            }
            result = new JObject { { "sounds", albumList } };
            WriteFile(AppConfig.SoundsPath, result.ToString());
        }

    }
}