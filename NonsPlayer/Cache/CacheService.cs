using Microsoft.Extensions.Logging;

namespace NonsPlayer.Cache;

using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Timers;


public class CacheService
{
    private readonly ConcurrentDictionary<string, CacheBase> _cache = new();
    private readonly Timer _cleanupTimer;
    public ILogger Logger = App.GetLogger<CacheService>();

    public CacheService() // 默认每分钟清理一次
    {
        var cleanupInterval = 60000;
        _cleanupTimer = new Timer(cleanupInterval);
        _cleanupTimer.Elapsed += (sender, e) => CleanupExpiredItems();
        _cleanupTimer.Start();
        Logger.LogInformation("CacheService init finished");
        Logger.LogInformation("Current cache expiration time(ms): {time}", cleanupInterval);
    }


    public bool TryGet<T>(string id, out T result)
    {
        if (_cache.TryGetValue(id, out var cacheItem) && !cacheItem.IsExpired())
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

    private void CleanupExpiredItems()
    {
        foreach (var key in _cache.Keys)
        {
            if (_cache.TryGetValue(key, out var cacheItem) && cacheItem.IsExpired())
            {
                _cache.TryRemove(key, out _);
            }
        }
    }
}