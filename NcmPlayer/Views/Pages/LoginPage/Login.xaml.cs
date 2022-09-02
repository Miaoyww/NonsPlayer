using NcmPlayer.Resources;
using NcmPlayer.Views.Pages.LoginPage;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Windows.Controls;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// Login.xaml 的交互逻辑
    /// </summary>
    public partial class Login : Page
    {
        private string tokenMD5 = string.Empty;
        public string Name = string.Empty;
        public Stream FaceStream = new MemoryStream();
        public string FaceUrl = string.Empty;
        public string Id = string.Empty;
        public int LastSignin = 0;
        public List<int> likelist = new List<int>();
        public static Login acc;

        public Login()
        {
            InitializeComponent();
            acc = this;
            DataContext = ResEntry.res;
            tokenMD5 = RegGeter.RegGet("Account", "TokenMD5").ToString();
            DecryptAndLogin(RegGeter.RegGet("Account", "Token").ToString());

            Name = RegGeter.RegGet("Account", "AccountName").ToString();
            FaceUrl = RegGeter.RegGet("Account", "AccountFaceUrl").ToString();
            Id = RegGeter.RegGet("Account", "AccountId").ToString();
            LastSignin = int.Parse(RegGeter.RegGet("Account", "LastSignin").ToString() ?? "0");
            string[] ids = Encoding.UTF8.GetString(Convert.FromBase64String(RegGeter.RegGet("Account", "Likelist").ToString())).Split(',');
            foreach (string id in ids)
            {
                if (!string.IsNullOrEmpty(id))
                {
                    likelist.Add(int.Parse(id));
                }
            }
            ResEntry.songInfo.LikeList = likelist;
            CheckLogin();
        }

        #region AES加密解密

        private static string AESEncrypt(string Data, string Key)
        {
            byte[] _aesKeyByte = { 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF };
            string _aesKeyStr = Encoding.UTF8.GetString(_aesKeyByte);
            Byte[] plainBytes = Encoding.UTF8.GetBytes(Data);
            Byte[] bKey = new Byte[32];
            Array.Copy(Encoding.UTF8.GetBytes(Key.PadRight(bKey.Length)), bKey, bKey.Length);
            Byte[] bVector = new Byte[16];
            Array.Copy(Encoding.UTF8.GetBytes(_aesKeyStr.PadRight(bVector.Length)), bVector, bVector.Length);
            Byte[] Cryptograph = null; // 加密后的密文
            Rijndael Aes = Rijndael.Create();
            try
            {
                // 开辟一块内存流
                using (MemoryStream Memory = new MemoryStream())
                {
                    // 把内存流对象包装成加密流对象
                    using (CryptoStream Encryptor = new CryptoStream(Memory,
                    Aes.CreateEncryptor(bKey, bVector),
                    CryptoStreamMode.Write))
                    {
                        Encryptor.Write(plainBytes, 0, plainBytes.Length);
                        Encryptor.FlushFinalBlock();
                        Cryptograph = Memory.ToArray();
                    }
                }
            }
            catch
            {
                Cryptograph = null;
            }
            return Convert.ToBase64String(Cryptograph);
        }

        private static String AESDecrypt(string data, string key)
        {
            byte[] _aesKeyByte = { 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF };
            string _aesKeyStr = Encoding.UTF8.GetString(_aesKeyByte);
            Byte[] encryptedBytes = Convert.FromBase64String(data);
            Byte[] bKey = new Byte[32];
            Array.Copy(Encoding.UTF8.GetBytes(key.PadRight(bKey.Length)), bKey, bKey.Length);
            Byte[] bVector = new Byte[16];
            Array.Copy(Encoding.UTF8.GetBytes(_aesKeyStr.PadRight(bVector.Length)), bVector, bVector.Length);
            Byte[] original = null; // 解密后的明文
            Rijndael Aes = Rijndael.Create();
            try
            {
                // 开辟一块内存流，存储密文
                using (MemoryStream Memory = new MemoryStream(encryptedBytes))
                {
                    // 把内存流对象包装成加密流对象
                    using (CryptoStream Decryptor = new CryptoStream(Memory,
                    Aes.CreateDecryptor(bKey, bVector),
                    CryptoStreamMode.Read))
                    {
                        // 明文存储区
                        using (MemoryStream originalMemory = new MemoryStream())
                        {
                            Byte[] Buffer = new Byte[1024];
                            Int32 readBytes = 0;
                            while ((readBytes = Decryptor.Read(Buffer, 0, Buffer.Length)) > 0)
                            {
                                originalMemory.Write(Buffer, 0, readBytes);
                            }
                            original = originalMemory.ToArray();
                        }
                    }
                }
            }
            catch
            {
                original = null;
            }
            return Encoding.UTF8.GetString(original);
        }

        #endregion AES加密解密

        #region 加密解密 Token

        private string machineCode = string.Empty;

        private string MachineCode
        {
            get
            {
                if (string.IsNullOrEmpty(machineCode))
                {
                    machineCode = NcmApi.Security.FingerPrint.Value().Replace("-", "");
                }
                return machineCode;
            }
        }

        public string GetKey(string data)
        {
            string key = string.Empty;
            if (MachineCode.Length < data.Length)
            {
                key = string.Concat(Enumerable.Repeat(MachineCode, (data.Length / MachineCode.Length)));
                key += MachineCode.Substring(0, data.Length - key.Length);
            }
            else if (MachineCode.Length > data.Length)
            {
                key = MachineCode.Substring(0, MachineCode.Length - data.Length);
            }
            else
            {
                key = MachineCode;
            }
            return key;
        }

        public string Encrypt(string data)
        {
            string codeMD5 = Convert.ToBase64String(MD5.HashData(Encoding.UTF8.GetBytes(data)));
            Regediter.Regedit("Account", "TokenMD5", codeMD5);
            string dataB64 = Convert.ToBase64String(Encoding.UTF8.GetBytes(data));
            string result = AESEncrypt(dataB64, GetKey(dataB64));
            Regediter.Regedit("Account", "Token", result);
            return result;
        }

        public void DecryptAndLogin(string data)
        {
            if (!string.IsNullOrEmpty(tokenMD5))
            {
                byte[] dataBytes = Convert.FromBase64String(AESDecrypt(data, GetKey(data)));

                string result = Encoding.UTF8.GetString(dataBytes);

                if (Convert.ToBase64String(MD5.HashData(Encoding.UTF8.GetBytes(result))).Equals(tokenMD5))
                {
                    ResEntry.ncm.Login(result);
                }
                else
                {
                    if (MainWindow.acc.IsLoaded)
                    {
                        PublicMethod.ShowDialog("token错误", "警告");
                    }
                }
            }
        }

        #endregion 加密解密 Token

        public void CheckLogin()
        {
            if (ResEntry.ncm.isLoggedin)
            {
                frame.Content = new Loggedin();
            }
            else
            {
                frame.Content = new UnLoggedin();
            }
        }
    }
}