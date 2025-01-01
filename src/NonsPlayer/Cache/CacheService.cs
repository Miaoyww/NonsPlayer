using Microsoft.Extensions.Logging;

namespace NonsPlayer.Cache;

using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Timers;


public class CacheService
{
    private readonly ConcurrentDictionary<string, CacheBase> _cache = new();
    public ILogger Logger = App.GetLogger<CacheService>();

    public CacheService()
    {
        Logger.LogInformation("CacheService init finished");
    }


    public bool TryGet<T>(string id, out T result)
    {
        if (_cache.TryGetValue(id, out var cacheItem))
        {
            result = (T)cacheItem.Value;
            return true;
        }

        result = default;
        return false;
    }

    public void AddOrUpdate<T>(string id, T value)
    {
        var cacheItem = new CacheBase(value);
        _cache.AddOrUpdate(id, cacheItem, (k, v) => cacheItem);

    }

    public bool TrySet(string id, CacheBase value)
    {
        if (_cache.ContainsKey(id))
        {
            _cache[id] = value;
            return true;
        }

        return _cache.TryAdd(id, value);
    }

    public void Remove(string key)
    {
        if (!_cache.ContainsKey(key)) return;

        try
        {
            _cache.Remove(key, out var result);
        }
        catch (Exception e)
        {
            return;
        }
    }
}