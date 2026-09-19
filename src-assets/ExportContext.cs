using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using LazyBearTechnology;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using UnityEngine;

namespace Gk2.AssetExporter
{
    public sealed class ExportContext : IDisposable
    {
        public readonly string DirectoryPath;
        public readonly SpriteExporter Sprites;
        public readonly List<object> Warnings = new List<object>();
        public readonly List<string> CompletedModules = new List<string>();
        public readonly SortedDictionary<string, string> English = new SortedDictionary<string, string>(StringComparer.Ordinal);
        public readonly HashSet<string> RequiredFontIcons = new HashSet<string>(StringComparer.Ordinal);
        public readonly SortedDictionary<string, List<object>> FontIcons = new SortedDictionary<string, List<object>>(StringComparer.Ordinal);
        public readonly List<object> SpriteAssets = new List<object>();

        public ExportContext(string directory)
        {
            DirectoryPath = directory;
            Directory.CreateDirectory(directory);
            Sprites = new SpriteExporter(this);
            // Load English independently, without changing the user's current language.
            var locale = Resources.Load<LL>("Locales/lng_en");
            if (locale == null) throw new InvalidOperationException("English locale resource is unavailable. Wait for the main menu.");
            for (int i = 0; i < locale.TextEntryCount; i++) English[locale.GetTextIdAt(i)] = locale.GetTextWithMarkup(i);
        }

        public string Localize(string key)
        {
            return key != null && English.TryGetValue(key, out var text) ? text : key;
        }

        public string FontIcon(string name)
        {
            if (!string.IsNullOrEmpty(name)) RequiredFontIcons.Add(name);
            return name;
        }

        public void Warn(string scope, string message) { Warnings.Add(new { scope, message }); }

        public void Write(string name, object data)
        {
            File.WriteAllText(Path.Combine(DirectoryPath, name), JsonConvert.SerializeObject(data, Formatting.Indented), new UTF8Encoding(false));
        }

        public void Finish()
        {
            foreach (var name in RequiredFontIcons.OrderBy(x => x, StringComparer.Ordinal))
                if (!FontIcons.ContainsKey(name)) Warn("font-icon", "Unresolved sprite tag: " + name);
            Write("icons.json", new { images = Sprites.Images, sprites = Sprites.Sprites, fontIcons = FontIcons, spriteAssets = SpriteAssets });
            Write("localization.en.json", English);
            Write("manifest.json", new
            {
                schemaVersion = 1, exporterVersion = "0.1.0", gameVersion = Application.version,
                unityVersion = Application.unityVersion, product = Application.productName,
                gameAssemblyMvid = typeof(ItemDef).Module.ModuleVersionId.ToString(),
                exportedAtUtc = DateTime.UtcNow.ToString("O"), locale = "en",
                status = Warnings.Count == 0 ? "complete" : "partial",
                completedModules = CompletedModules, warnings = Warnings
            });
        }

        public void Dispose() { Sprites.Dispose(); }
    }

    // Capture data fields, never property getters or executable expressions. This preserves
    // definitions without running gameplay actions or exporting runtime caches/object graphs.
    internal static class DefinitionFields
    {
        internal static IEnumerable<FieldInfo> Fields(Type type)
        {
            // Include private serialized fields such as ItemDef.customIcon, excluding caches.
            return type.GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)
                .Where(f => f.IsPublic || f.IsDefined(typeof(SerializeField), true))
                .OrderBy(f => f.Name, StringComparer.Ordinal);
        }

        public static JObject Read(object definition)
        {
            var result = new JObject();
            foreach (var field in Fields(definition.GetType()))
                result[field.Name] = Value(field.GetValue(definition));
            return result;
        }

        private static JToken Value(object value)
        {
            if (value == null) return JValue.CreateNull();
            if (value is LazyExpression expression) return new JObject { ["expression"] = expression.GetRawExpressionString() };
            if (value is ExpressionGameRes resource) return new JObject { ["type"] = resource.name, ["expression"] = resource.expression?.GetRawExpressionString() };
            if (value is string || value is bool || value.GetType().IsPrimitive || value is decimal) return new JValue(value);
            if (value.GetType().IsEnum) return new JValue(value.ToString());
            if (value is System.Collections.IEnumerable list) return new JArray(list.Cast<object>().Select(Value));
            // Unknown future field types remain explicit rather than silently evaluating them.
            return new JObject { ["unsupportedType"] = value.GetType().FullName };
        }
    }
}
