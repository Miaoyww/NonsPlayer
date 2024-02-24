using System.Reflection;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;

namespace NonsPlayer.Core.Services;


public class AdapterService
{
    public List<object> LoadPlugins(string directory)
    {
        var plugins = new List<object>();
        string[] dllFiles = Directory.GetFiles(directory, "*.dll");

        foreach (var file in dllFiles)
        {
            try
            {
                var assembly = Assembly.LoadFrom(file);
                var types = assembly.GetTypes().Where(t => typeof(IAdapterService).IsAssignableFrom(t));

                foreach (var type in types)
                {
                    var plugin = Activator.CreateInstance(type);
                    plugins.Add(plugin);
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error loading plugins from '{file}': {ex.Message}");
            }
        }

        return plugins;
    }
}