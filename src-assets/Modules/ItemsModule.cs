using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using Newtonsoft.Json.Linq;

namespace Gk2.AssetExporter
{
    public sealed class ItemsModule : IExportModule
    {
        public string Id => "items";
        public int Order => 10;

        public IEnumerator Export(ExportContext context)
        {
            var definitions = GameBalance.Me.itemDefs.OrderBy(d => d.id, StringComparer.Ordinal).ToArray();
            if (definitions.Length == 0) throw new InvalidOperationException("No item definitions loaded");
            var items = new SortedDictionary<string, object>(StringComparer.Ordinal);
            foreach (var definition in definitions)
            {
                try
                {
                    var star = definition.qualityType == ItemDef.QualityType.Star;
                    var baseId = star ? definition.id.Split(':')[0] : definition.id;
                    var nameKey = star && !context.English.ContainsKey(definition.id) ? baseId : definition.id;
                    var descriptionKey = star && !context.English.ContainsKey(definition.id + "_d") ? baseId + "_d" : definition.id + "_d";
                    if (!context.English.ContainsKey(nameKey)) context.Warn("item:" + definition.id, "Missing English name: " + nameKey);
                    if (string.IsNullOrEmpty(definition.iconId)) context.Warn("item:" + definition.id, "Empty inventory icon ID");
                    var iconNames = new HashSet<string>(StringComparer.Ordinal);
                    void Icon(string name) { if (!string.IsNullOrEmpty(name)) iconNames.Add(context.FontIcon(name)); }
                    Icon(definition.qualityIcon);
                    Icon(definition.talentType);
                    foreach (var talent in definition.talentIds) Icon(talent);
                    if (definition.redSkulls != 0 || definition.redSkullsMaxCollar != 0) Icon("rskull");
                    if (definition.whiteSkulls != 0 || definition.whiteSkullsMaxCollar != 0) Icon("skull");
                    if (definition.type == ItemType.BodyArmor) Icon("equip_icon_armor");
                    if (definition.isWeapon) Icon(definition.type == ItemType.Bow ? "equip_icon_arrow" : definition.type == ItemType.Pike ? "squad_equip_icon-spear" : "equip_icon_sword");
                    Icon("rune_r"); Icon("rune_g"); Icon("rune_b");
                    var useEffects = definition.gameResOnUse.Select(effect => new
                    {
                        resource = effect.name,
                        expression = effect.expression?.GetRawExpressionString(),
                        iconName = context.FontIcon(GameResDisplayConfig.GetConfigForRes(effect.name, GameResIconType.Common)?.iconName ?? effect.name)
                    }).ToArray();
                    foreach (var effect in useEffects) Icon(effect.iconName);
                    // Record literal perk references for tooltip use, but never evaluate onUseExpressions.
                    var perkIds = new HashSet<string>(StringComparer.Ordinal);
                    foreach (var expression in definition.onUseExpressions ?? new List<LazyExpression>())
                    {
                        var text = expression.GetRawExpressionString() ?? "";
                        foreach (Match match in Regex.Matches(text, "\\bAddPerk\\s*\\(\\s*\"([^\"]+)\"")) perkIds.Add(match.Groups[1].Value);
                    }
                    var relatedPerkIds = new HashSet<string>(perkIds, StringComparer.Ordinal);
                    if (!string.IsNullOrEmpty(definition.bodyLinkedPerk)) relatedPerkIds.Add(definition.bodyLinkedPerk);
                    var perks = relatedPerkIds.OrderBy(x => x, StringComparer.Ordinal).Select(id =>
                    {
                        var perk = GameBalance.Me.GetDataOrNull<PerkDef>(id);
                        return new { id, onUse = perkIds.Contains(id), bodyLinked = id == definition.bodyLinkedPerk,
                            name = context.Localize(id), description = context.Localize(id + "_d"), sprite = perk == null ? null : context.Sprites.Sprite(perk.IconId) };
                    }).ToArray();
                    // All public definition fields are retained, including raw conditions,
                    // bag dimensions, durability, rune expressions, prices and sorting data.
                    var fields = DefinitionFields.Read(definition);
                    foreach (var unsupported in fields.Descendants().OfType<JProperty>().Where(p => p.Name == "unsupportedType"))
                        context.Warn("item:" + definition.id, "Unsupported definition field at " + unsupported.Path + ": " + unsupported.Value);
                    items.Add(definition.id, new
                    {
                        id = definition.id, nameKey, name = context.Localize(nameKey), descriptionKey,
                        description = context.Localize(descriptionKey), sprite = context.Sprites.Sprite(definition.iconId),
                        quality = new { type = definition.qualityType.ToString(), value = definition.quality, family = star ? baseId : null,
                            overlaySprite = star ? context.Sprites.Sprite("item_star_" + definition.quality) : null,
                            effectIconName = definition.qualityIcon },
                        useEffects, relatedPerks = perks, fontIconNames = iconNames.OrderBy(x => x, StringComparer.Ordinal).ToArray(),
                        fields
                    });
                }
                catch (Exception error) { context.Warn("item:" + definition.id, error.ToString()); }
                yield return null;
            }
            var families = definitions.Where(d => d.qualityType == ItemDef.QualityType.Star)
                .GroupBy(d => d.id.Split(':')[0]).OrderBy(g => g.Key, StringComparer.Ordinal)
                .ToDictionary(g => g.Key, g => g.OrderBy(d => d.quality).ThenBy(d => d.id, StringComparer.Ordinal)
                    .Select(d => new { id = d.id, quality = d.quality, overlaySprite = "item_star_" + d.quality }).ToArray());
            context.Write("items.json", items);
            context.Write("quality-families.json", families);
            context.Write("item-ids.json", definitions.Select(d => d.id).ToArray());
        }
    }
}
