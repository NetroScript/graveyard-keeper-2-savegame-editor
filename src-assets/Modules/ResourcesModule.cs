using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;

namespace Gk2.AssetExporter
{
    public sealed class ResourcesModule : IExportModule
    {
        public string Id => "resources";
        public int Order => 20;
        public IEnumerator Export(ExportContext context)
        {
            // Enumerate the actual table, including types added in later versions. This is
            // the only private game field dependency; failure is recorded in the manifest.
            var field = typeof(GameResDisplayConfig).GetField("datas", BindingFlags.Instance | BindingFlags.NonPublic);
            if (field == null) throw new MissingFieldException("GameResDisplayConfig.datas changed");
            var data = (List<GameResDisplayData>)field.GetValue(GameResDisplayConfig.Instance);
            var records = data.OrderBy(d => d.atomType, StringComparer.Ordinal).Select(d => new
            {
                resource = d.atomType,
                configurations = d.iconConfigs.Select(c => new { iconType = c.iconType.value, iconName = context.FontIcon(c.iconName) }).ToArray()
            }).ToArray();
            foreach (var name in new[] { "tech_red", "tech_green", "tech_blue", "happiness", "rune_r", "rune_g", "rune_b" }) context.FontIcon(name);
            context.Write("resources.json", records);
            yield break;
        }
    }
}
