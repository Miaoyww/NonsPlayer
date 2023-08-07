using System.Diagnostics;

namespace NonsPlayer.Cache;

public class CacheManager
{
    private readonly Dictionary<string, object> _cache = new();

    private readonly Dictionary<string, int> _refCount = new();

    public static CacheManager Instance { get; } = new();

    /// <summary>
    ///     尝试获取缓存资源
    /// </summary>
    /// <param name="key"></param>
    /// <typeparam name="T"></typeparam>
    /// <returns>不存在返回null</returns>
    public CacheItem<T>? TryGet<T>(string key) where T : class
    {
        if (_cache.TryGetValue(key, out var value))
        {
            if (_refCount.TryGetValue(key, out var count))
                _refCount[key] = count + 1;
            else
                _refCount.Add(key, 1);

            return value as CacheItem<T>;
        }

        return null;
    }

    public async Task<CacheItem<T>?>? TryGetAsync<T>(string key) where T : class
    {
        return await Task.Run(() => TryGet<T>(key));
    }

    public void Add<T>(string key, T value)
    {
        if (_cache.ContainsKey(key)) return;

        _cache.Add(key, new CacheItem<T>
        {
            Data = value
        });
        _refCount.Add(key, 0);
    }

    public void Set<T>(string key, CacheItem<T> value)
    {
        if (_cache.ContainsKey(key))
        {
            _cache[key] = value;
            return;
        }

        _cache.Add(key, value);
        _refCount.Add(key, 0);
    }

    public void Remove(string key)
    {
        if (!_cache.ContainsKey(key)) return;

        try
        {
            _cache.Remove(key);
        }
        catch (Exception e)
        {
            Debug.WriteLine($"尝试删除{key}时出现了错误, 可能是由于它依然被控件所引用造成的{e}");
            return;
        }

        _refCount.Remove(key);
        Debug.WriteLine($"已删除缓存{key}");
    }

    public void Clear()
    {
        _cache.Clear();
        _refCount.Clear();
    }
}