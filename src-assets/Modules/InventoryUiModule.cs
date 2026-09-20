using System.Collections;
using System.Linq;
using UnityEngine;

namespace Gk2.AssetExporter
{
    // UIItemCell's background is a prefab reference, not an item icon ID.
    // Export loaded cells without constructing UI objects or changing their state.
    public sealed class InventoryUiModule : IExportModule
    {
        public string Id => "inventory-ui";
        public int Order => 35;
        public IEnumerator Export(ExportContext context)
        {
            var backgrounds = Resources.FindObjectsOfTypeAll<UIItemCell>()
                .Where(c => c.Background != null && c.Background.sprite != null)
                .GroupBy(c => c.Background.sprite)
                .OrderByDescending(g => g.Count()).ThenBy(g => g.Key.name).ToArray();
            if (backgrounds.Length == 0)
            {
                context.Warn(Id, "No inventory cell is loaded. Open the character inventory and export again.");
                yield break;
            }
            var sprites = backgrounds.Select(g => new {
                sprite = context.Sprites.Sprite("inventory-ui/" + g.Key.name, g.Key),
                border = new { left = g.Key.border.x, bottom = g.Key.border.y, right = g.Key.border.z, top = g.Key.border.w }
            }).ToArray();
            context.Write("inventory-ui.json", new { backgroundSprite = sprites[0].sprite, backgrounds = sprites });
            yield return null;
        }
    }
}
