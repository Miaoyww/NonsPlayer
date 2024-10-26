using NonsPlayer.Core.Services;
using SixLabors.ImageSharp;
using SixLabors.ImageSharp.Formats.Jpeg;
using SixLabors.ImageSharp.Processing;

namespace NonsPlayer.Core.Utils;

public static class LocalUtils

{
    public static bool IsMusic(string path)
    {
        string extension = Path.GetExtension(path);
        if (string.IsNullOrEmpty(extension)) return false;
        string[] songEx = [".wav", ".flac", ".ape", ".m4a", ".caf", ".wv", ".mp3", ".aac", ".ogg", ".opus"];
        foreach (string s in songEx)
        {
            if (extension.Equals(s)) return true;
        }

        return false;
    }

    public static byte[]? CompressAndConvertToByteArray(byte[]? imageData, int width, int height)
    {
        try
        {
            if (imageData == null) return null;

            using var ms = new MemoryStream(imageData); 
            Image image = Image.Load(ms);
            image.Mutate(x => x.Resize(width, height));
            using var msOutput = new MemoryStream();
            image.Save(msOutput, new JpegEncoder { Quality = 20 });
            return msOutput.ToArray();
        }
        catch (NotSupportedException ex)
        {
            ExceptionService.Instance.Throw("Not Supported ImageFormat: " + ex.Message);
        }
        catch (UnknownImageFormatException ex)
        {
            ExceptionService.Instance.Throw("Unknown ImageFormat: " + ex.Message);
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
        }
        return null;
    }

    public static byte[]? CompressAndConvertToByteArray(Stream stream, int width, int height)
    {
        try
        {
            using var ms = stream;
            Image image = Image.Load(ms);
            image.Mutate(x => x.Resize(width, height));
            using var msOutput = new MemoryStream();
            image.Save(msOutput, new JpegEncoder { Quality = 20 });
            return msOutput.ToArray();
        }
        catch (NotSupportedException ex)
        {
            ExceptionService.Instance.Throw("Not Supported ImageFormat: " + ex.Message);
        }
        catch (UnknownImageFormatException ex)
        {
            ExceptionService.Instance.Throw("Unknown ImageFormat: " + ex.Message);
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
        }
        return null;
    }
}