using Newtonsoft.Json;

namespace NonsPlayer.Core.Helpers;

public static class JsonUtils
{
    public static async Task<T> ToObjectAsync<T>(string value)
    {
        return await Task.Run(() => { return JsonConvert.DeserializeObject<T>(value); });
    }

    public static async Task<string> StringifyAsync(object value)
    {
        return await Task.Run(() => { return JsonConvert.SerializeObject(value); });
    }
}