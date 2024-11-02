using System.Text;
using Newtonsoft.Json;
using NonsPlayer.Core.Contracts.Services;
using JsonSerializer = System.Text.Json.JsonSerializer;

namespace NonsPlayer.Core.Services;

public class FileService
{
    public T Read<T>(string folderPath, string fileName)
    {
        var path = Path.Combine(folderPath, fileName);
        if (File.Exists(path))
        {
            var data = File.ReadAllText(path);
            return JsonConvert.DeserializeObject<T>(data);
        }

        return default;
    }

    public string ReadData(string filename)
    {
        var path = Path.Combine(ConfigManager.Instance.Settings.Data, filename);
        if (!File.Exists(path))
        {
            File.Create(path).Dispose();
            return string.Empty;
        }

        using var reader = new StreamReader(path);
        var json = reader.ReadToEnd();
        return json;
    }

    public void SaveData(string fileName, string content)
    {
        var path = Path.Combine(ConfigManager.Instance.Settings.Data, fileName);

        if (!Directory.Exists(path)) Directory.CreateDirectory(ConfigManager.Instance.Settings.Data);
        File.WriteAllText(path, content, Encoding.UTF8);
    }

    public void Save<T>(string folderPath, string fileName, T content)
    {
        if (!Directory.Exists(folderPath)) Directory.CreateDirectory(folderPath);

        var fileContent = JsonConvert.SerializeObject(content);
        File.WriteAllText(Path.Combine(folderPath, fileName), fileContent, Encoding.UTF8);
    }

    public void Delete(string filePath)
    {
        if (filePath != null && File.Exists(filePath))
        {
            File.Delete(filePath);
        }
    }

    public void DeleteFolder(string folderPath)
    {
        if (!Directory.Exists(folderPath))
        {
            return;
        }

        try
        {
            string[] files = Directory.GetFiles(folderPath, "*", SearchOption.TopDirectoryOnly);
            foreach (string file in files)
            {
                Delete(file);
            }
        }
        catch (UnauthorizedAccessException ex)
        {
            ExceptionService.Instance.Throw(ex);
        }
        catch (IOException ex)
        {
            // ignore
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
        }
    }
    public void Move<T>(string targetFile, string folderPath)
    {
    }

    public void WriteData(string filename, string content)
    {
        var path = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, filename);
        using (var writer = new StreamWriter(path))
        {
            writer.Write(content);
        }
    }

    public static bool Exist(string path)
    {
        if (File.Exists(path))
        {
            return true;
            
        }
        else if (Directory.Exists(path))
        {
            return true;
        }

        return false;
    }
    public static long GetFileSize(string filePath)
    {
        if (File.Exists(filePath))
        {
            FileInfo fileInfo = new FileInfo(filePath);
            return fileInfo.Length;
        }

        return 0;

    }
    public static long GetDirectorySize(string folderPath)
    {
        if (!Directory.Exists(folderPath))
        {
            return 0;
        }

        long size = 0;

        foreach (string file in Directory.GetFiles(folderPath, "*", SearchOption.AllDirectories))
        {
            size += new FileInfo(file).Length;
        }

        return size;
    }
    public static string FormatSize(long bytes)
    {
        string[] sizes = { "B", "KB", "MB", "GB", "TB" };
        int order = 0;
        double size = bytes;

        while (size >= 1024 && order < sizes.Length - 1)
        {
            order++;
            size /= 1024;
        }

        return $"{size:0.##} {sizes[order]}";
    }
}
