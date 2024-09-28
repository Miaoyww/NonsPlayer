using NonsPlayer.Core.Services;
using System.Collections.Concurrent;

namespace NonsPlayer.Services;

public class GlobalThreadPool
{
    public static GlobalThreadPool Instance { get; } = new(16);
    private readonly BlockingCollection<Action> _taskQueue = new();
    private readonly Thread[] _workers;

    public GlobalThreadPool(int numberOfThreads)
    {
        _workers = new Thread[numberOfThreads];
        for (int i = 0; i < numberOfThreads; i++)
        {
            _workers[i] = new Thread(Worker);
            _workers[i].Start();
        }
    }

    private void Worker()
    {
        foreach (var task in _taskQueue.GetConsumingEnumerable())
        {
            task();
        }
    }

    public void Enqueue(Action task)
    {
        _taskQueue.Add(task);
    }

    public void Shutdown()
    {
        _taskQueue.CompleteAdding();
        foreach (var worker in _workers)
        {
            worker.Join();
        }
    }
}