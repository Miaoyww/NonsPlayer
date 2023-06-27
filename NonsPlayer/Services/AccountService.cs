using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Security.Cryptography;
using System.Text;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsApi;
using NonsPlayer.Helpers;
using NonsPlayer.Heplers;

namespace NonsPlayer.Services;

public class AccountService : INotifyPropertyChanged
{
    private string _tokenMd5 = string.Empty;
    private string _machineCode = string.Empty;
    private string _name = "未登录";

    private ImageBrush _face = new ImageBrush
    {
        ImageSource =
            new BitmapImage(new Uri("ms-appdata:///Assets/UnKnowResource.png"))
    };

    public string Uid
    {
        get;
        set;
    }

    public string Name
    {
        get => _name;
        set
        {
            _name = value;
            OnPropertyChanged(nameof(Name));
        }
    }

    public ImageBrush Face
    {
        get => _face;
        set
        {
            _face = value;
            OnPropertyChanged(nameof(Face));
        }
    }

    public static AccountService Instance
    {
        get;
    } = new();

    public void LoginByToken(string token)
    {
        var result = Encrypt(token);

        RegHelper.Instance.Set(RegHelper.Regs.AccountToken, result[0]);
        RegHelper.Instance.Set(RegHelper.Regs.AccountTokenMd5, result[1]);
        Nons.Instance.Login(token);
        InitAccount();
    }

    public void LoginByReg()
    {
        // 读取注册表中的Token，若存在进行登陆操作，否则return
        string data = (string)RegHelper.Instance.Get(RegHelper.Regs.AccountToken, string.Empty);
        _tokenMd5 = (string)RegHelper.Instance.Get(RegHelper.Regs.AccountTokenMd5, string.Empty);
        if (string.IsNullOrEmpty(data) || string.IsNullOrEmpty(_tokenMd5))
        {
            return;
        }

        byte[] dataBytes = Convert.FromBase64String(AESDecrypt(data, GetKey(data)));

        string result = Encoding.UTF8.GetString(dataBytes);

        if (Convert.ToBase64String(MD5.HashData(Encoding.UTF8.GetBytes(result))).Equals(_tokenMd5))
        {
            Nons.Instance.Login(result);
            InitAccount();
        }
    }

    public async void InitAccount()
    {
        var result = await Api.User.Account(Nons.Instance);
        if (result is null)
        {
            return;
        }

        Uid = result["profile"]["userId"].ToString();
        Name = result["profile"]["nickname"].ToString();
        Face = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(result["profile"]["avatarUrl"].ToString()))
        };
        UserPlaylistHelper.Instance.Init();
    }

    #region AES加密解密

    private string AESEncrypt(string Data, string Key)
    {
        byte[] _aesKeyByte =
            {0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF};
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

    private string AESDecrypt(string data, string key)
    {
        byte[] _aesKeyByte =
            {0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF};
        string _aesKeyStr = Encoding.UTF8.GetString(_aesKeyByte);
        var encryptedBytes = Convert.FromBase64String(data);
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

    private string MachineCode
    {
        get
        {
            if (string.IsNullOrEmpty(_machineCode))
            {
                _machineCode = Framework.Api.FingerPrint.Value().Replace("-", "");
            }

            return _machineCode;
        }
    }

    private string GetKey(string data)
    {
        string key;
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

    /// <summary>
    /// 
    /// </summary>
    /// <param name="data"></param>
    /// <returns>[0]是加密结果, [1]是MD5</returns>
    private string[] Encrypt(string data)
    {
        var codeMD5 = Convert.ToBase64String(MD5.HashData(Encoding.UTF8.GetBytes(data)));
        var dataB64 = Convert.ToBase64String(Encoding.UTF8.GetBytes(data));
        var result = AESEncrypt(dataB64, GetKey(dataB64));
        return new string[2] { result, codeMD5 };
    }

    #endregion 加密解密 Token

    public event PropertyChangedEventHandler? PropertyChanged;

    protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}