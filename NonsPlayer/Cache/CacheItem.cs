namespace NonsPlayer.Cache;

public class CacheItem<T>
{
    public T Data;

    public DateTime ExpirationTime;

    public CacheItem(DateTime? ExpirationTime = null)
    {
        this.ExpirationTime = ExpirationTime ?? DateTime.Now.AddMinutes(1);
    }
}