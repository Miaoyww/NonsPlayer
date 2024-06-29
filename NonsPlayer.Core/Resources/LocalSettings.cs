using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Serialization;

namespace NonsPlayer.Core.Resources
{
    public class LocalSettings
    {
        [JsonIgnore] public string DataPath { get; set; }
        [JsonIgnore] public string ConfigFilePath;
        [JsonPropertyName("adapter_path")] public string AdapterPath { get; set; }
        [JsonPropertyName("plugin_path")] public string PluginPath { get; set; }
        [JsonPropertyName("data_path")] public string Data { get; set; }
        [JsonPropertyName("theme")] public string Theme { get; set; }

        [JsonPropertyName("disabled_adapters")]
        public string DisabledAdapters { get; set; }

        [JsonPropertyName("disabled_plugins")] public string DisabledPlugins { get; set; }

        public LocalSettings()
        {
            DataPath = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData) + "/NonsPlayer";
            ConfigFilePath = DataPath + "/config.json";
            AdapterPath = DataPath + "/Adapters";
            PluginPath = DataPath + "/Plugins";
            Data = DataPath + "/Data";
            Theme = "Light";
            DisabledAdapters = "";
            DisabledPlugins = "";
        }
    }
}