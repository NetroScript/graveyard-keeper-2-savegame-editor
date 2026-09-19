using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;

namespace Gk2.AssetExporter
{
    public sealed class InventoryRulesModule : IExportModule
    {
        public string Id => "inventory-rules";
        public int Order => 30;

        public IEnumerator Export(ExportContext context)
        {
            var definitions = GameBalance.Me.itemDefs.OrderBy(d => d.id, StringComparer.Ordinal).ToArray();
            var bags = new SortedDictionary<string, object>(StringComparer.Ordinal);
            foreach (var bag in definitions.Where(d => d.isBag))
            {
                var allowed = new List<string>();
                bool complete = true;
                foreach (var item in definitions)
                {
                    try
                    {
                        // This verified method only checks isBag and group membership. Calling
                        // it captures the game's rules without duplicating enum/range assumptions.
                        if (item.CanBeInsertedInBag(bag)) allowed.Add(item.id);
                    }
                    catch (Exception error)
                    {
                        complete = false;
                        context.Warn("bag:" + bag.id + "/" + item.id, error.Message);
                    }
                }
                bags[bag.id] = new { complete, allowedItemIds = allowed };
                yield return null;
            }
            var equipment = definitions.Select(d => new
            {
                id = d.id, canBeEquipped = d.CanItemBeEquipped(), isFightingEquipment = d.IsFightingEquipment(),
                type = d.type.ToString(), typeValue = (int)d.type
            }).ToArray();
            context.Write("inventory-rules.json", new
            {
                ruleVersion = 1, bags, equipment,
                scope = "Definition compatibility only; capacity, per-instance filters and slot rules must also be checked against the save."
            });
            context.Write("item-definition-schema.json", new
            {
                type = typeof(ItemDef).FullName,
                fields = DefinitionFields.Fields(typeof(ItemDef)).Select(f => new { name = f.Name, type = f.FieldType.FullName, isPublic = f.IsPublic }).ToArray(),
                itemTypes = Enum.GetValues(typeof(ItemType)).Cast<ItemType>().Select(t => new { name = t.ToString(), value = (int)t }).ToArray(),
                itemSizes = Enum.GetValues(typeof(ItemSize)).Cast<ItemSize>().Select(t => new { name = t.ToString(), value = (int)t }).ToArray()
            });
        }
    }
}
