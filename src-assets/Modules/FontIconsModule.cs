using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using TMPro;
using UnityEngine;

namespace Gk2.AssetExporter
{
    public sealed class FontIconsModule : IExportModule
    {
        public string Id => "font-icons";
        public int Order => 1000;

        public IEnumerator Export(ExportContext context)
        {
            var queue = new Queue<TMP_SpriteAsset>();
            var seen = new HashSet<int>();
            if (TMP_Settings.defaultSpriteAsset != null) queue.Enqueue(TMP_Settings.defaultSpriteAsset);
            foreach (var asset in Resources.LoadAll<TMP_SpriteAsset>(TMP_Settings.defaultSpriteAssetPath ?? "Sprite Assets/")) queue.Enqueue(asset);
            foreach (var text in Resources.FindObjectsOfTypeAll<TMP_Text>()) if (text.spriteAsset != null) queue.Enqueue(text.spriteAsset);
            foreach (var asset in Resources.FindObjectsOfTypeAll<TMP_SpriteAsset>().OrderBy(a => a.name, StringComparer.Ordinal)) queue.Enqueue(asset);
            int sequence = 0;
            while (queue.Count > 0)
            {
                var asset = queue.Dequeue();
                if (asset == null || !seen.Add(asset.GetInstanceID())) continue;
                var assetId = "tmp-" + sequence++;
                var fallbackNames = (asset.fallbackSpriteAssets ?? new List<TMP_SpriteAsset>()).Where(a => a != null).Select(a => a.name).ToArray();
                foreach (var fallback in asset.fallbackSpriteAssets ?? new List<TMP_SpriteAsset>()) if (fallback != null) queue.Enqueue(fallback);
                context.SpriteAssets.Add(new { id = assetId, name = asset.name, isDefault = asset == TMP_Settings.defaultSpriteAsset, fallbackNames });
                foreach (var character in asset.spriteCharacterTable)
                {
                    // Controller prompts are not used by the editor. Filter before GPU
                    // readback/PNG encoding so they produce neither metadata nor images.
                    if (IsControllerIcon(character.name)) continue;
                    try
                    {
                        var glyph = character.glyph;
                        if (glyph == null) throw new InvalidOperationException("Missing glyph");
                        var rect = glyph.glyphRect;
                        var image = context.Sprites.Crop(asset.spriteSheet, rect.x, rect.y, rect.width, rect.height);
                        if (!context.FontIcons.TryGetValue(character.name, out var entries))
                            context.FontIcons[character.name] = entries = new List<object>();
                        // Preserve collisions across assets instead of silently selecting the wrong icon.
                        entries.Add(new { assetId, assetName = asset.name, image, unicode = character.unicode,
                            scale = character.scale, glyphScale = glyph.scale, glyphIndex = glyph.index,
                            metrics = new { width = glyph.metrics.width, height = glyph.metrics.height,
                                bearingX = glyph.metrics.horizontalBearingX, bearingY = glyph.metrics.horizontalBearingY,
                                advance = glyph.metrics.horizontalAdvance } });
                    }
                    catch (Exception error) { context.Warn("font-icon:" + asset.name + "/" + character.name, error.Message); }
                    yield return null;
                }
            }
            if (seen.Count == 0) context.Warn(Id, "No TMP sprite assets found; open an inventory and export again.");
        }

        private static bool IsControllerIcon(string name)
        {
            return name != null &&
                (name.StartsWith("xbox_", StringComparison.OrdinalIgnoreCase) ||
                 name.StartsWith("switch_", StringComparison.OrdinalIgnoreCase) ||
                 name.StartsWith("ps_", StringComparison.OrdinalIgnoreCase));
        }
    }
}
