﻿using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text.Json.Serialization;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Services;

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

        [JsonPropertyName("volume")] public double Volume { get; set; }

        [JsonPropertyName("adapter_account_tokens")]
        public Dictionary<string, string> AdapterAccountTokens { get; set; }

        [JsonPropertyName("default_adapter")] public string DefaultAdapter { get; set; }

        [JsonPropertyName("disabled_adapters")]
        public string DisabledAdapters { get; set; }

        [JsonPropertyName("disabled_plugins")] public string DisabledPlugins { get; set; }

        [JsonPropertyName("total_play_count")] public int TotalPlayCount { get; set; }

        [JsonPropertyName("log")] public string Log { get; set; }

        [JsonPropertyName("today_play_duration")]
        public Tuple<DateTime, TimeSpan> TodayPlayDuration { get; set; }

        [JsonPropertyName("local_artist_sep")] public List<string> LocalArtistSep { get; set; }

        public LocalSettings()
        {
            DataPath = Path.Join(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "NonsPlayer");
            ConfigFilePath = Path.Join(DataPath, "config.json");
            AdapterPath = Path.Join(DataPath, "adapters");
            PluginPath = Path.Join(DataPath, "plugins");
            Data = Path.Join(DataPath, "Data");
            Log = Path.Combine(DataPath, "logs", $"NonsPlayer_{DateTime.Now:yyMMdd_HHmmss}.log");
            Theme = "Light";
            Volume = 50;
            TotalPlayCount = 0;
            AdapterAccountTokens = new Dictionary<string, string>();
            DefaultAdapter = string.Empty;
            DisabledAdapters = string.Empty;
            DisabledPlugins = string.Empty;
            TodayPlayDuration = new Tuple<DateTime, TimeSpan>(DateTime.Now, TimeSpan.Zero);
            LocalArtistSep = [";", ","];
        }
    }
}