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

    public void Delete(string folderPath, string fileName)
    {
        if (fileName != null && File.Exists(Path.Combine(folderPath, fileName)))
            File.Delete(Path.Combine(folderPath, fileName));
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
}