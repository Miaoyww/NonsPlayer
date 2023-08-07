using System.Text;
using Newtonsoft.Json;
using NonsPlayer.Core.Contracts.Services;
using JsonSerializer = System.Text.Json.JsonSerializer;

namespace NonsPlayer.Core.Services;

public class FileService : IFileService
{
    public static FileService Instance { get; } = new();

    public T Read<T>(string folderPath, string fileName)
    {
        var path = Path.Combine(folderPath, fileName);
        if (File.Exists(path))
        {
            var json = File.ReadAllText(path);
            return JsonConvert.DeserializeObject<T>(json);
        }

        return default;
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

    public void WriteData(string filename, string content)
    {
        var path = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, filename);
        using (var writer = new StreamWriter(path))
        {
            writer.Write(content);
        }
    }

    public T ReadData<T>(string filename)
    {
        var path = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, filename);
        string json;
        using (var reader = new StreamReader(path))
        {
            json = reader.ReadToEnd();
        }

        var result = JsonSerializer.Deserialize<T>(json);
        return result;
    }
}