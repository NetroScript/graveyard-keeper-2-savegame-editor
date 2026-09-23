using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using LazyBearTechnology;

namespace Gk2.AssetExporter
{
    /// <summary>Exports the immutable definitions used by the technology and inspiration screens.</summary>
    public sealed class ProgressionModule : IExportModule
    {
        public string Id => "progression";
        public int Order => 25;

        private static object Localized(ExportContext context, string id)
        {
            return new
            {
                id,
                name = context.Localize(id),
                description = context.Localize(id + "_d")
            };
        }

        private static string Title(LinkedEntityWidgetData data)
        {
            var prefix = data.GetLinkedEntityPrefix();
            var header = data.GetLinkedEntityHeader();
            return string.IsNullOrEmpty(prefix) ? header : prefix + ": " + header;
        }

        private static object[] Ingredients(ExportContext context, IEnumerable<NeedItemData> needs)
        {
            return needs.Select(need =>
            {
                var item = need.groupType == ItemGroup.None ? need.ItemDef : null;
                if (item == null && need.TryGetGroupItemDefs(out var group)) item = group.FirstOrDefault();
                return (object)new
                {
                    id = need.Id,
                    name = item == null ? context.Localize(need.Id) : context.Localize(item.id),
                    count = need.GetCount(),
                    sprite = item == null ? null : context.Sprites.Sprite(item.iconId)
                };
            }).ToArray();
        }

        public IEnumerator Export(ExportContext context)
        {
            foreach (var icon in new[] { "tech_red", "tech_green", "tech_blue", "talent_orange", "talent_red", "talent_green", "talent_yellow", "talent_blue" })
                context.FontIcon(icon);

            var tabs = Enum.GetValues(typeof(TechTreeTab)).Cast<TechTreeTab>().Select(tab =>
            {
                var id = "tech_tab_" + tab;
                return new
                {
                    id = tab.ToString(),
                    name = context.Localize(id),
                    sprite = context.Sprites.Sprite(id)
                };
            }).ToArray();

            var technologies = new List<object>();
            foreach (var tech in GameBalance.Me.techDefs.OrderBy(x => x.tab).ThenBy(x => x.TreePos.x).ThenBy(x => x.TreePos.y))
            {
                try
                {
                    var rewards = new List<object>();
                    foreach (var id in tech.craftsAfterUnlock)
                    {
                        var def = GameBalance.GetCraftDef(id);
                        var item = def.TryGetResultingItemDef(true);
                        var linked = new LinkedEntityWidgetData(def, null);
                        rewards.Add(new { type = "craft", id, name = Title(linked), description = item == null ? context.Localize(id + "_d") : context.Localize(item.GetDescriptionLocale()), sprite = context.Sprites.Sprite(def.GetCraftResultIcon(null)), craftedAt = def.craftsIn.Select(context.Localize).ToArray(), ingredients = Ingredients(context, def.needItems) });
                    }
                    foreach (var id in tech.alchemyFormulasAfterUnlock)
                    {
                        var def = GameBalance.Me.GetDataOrNull<AlchemyFormulaDef>(id);
                        var item = def == null ? null : def.ItemDef;
                        var linked = def == null ? null : new LinkedEntityWidgetData(def, null);
                        rewards.Add(new { type = "alchemy", id, name = linked == null ? context.Localize(id) : Title(linked), description = context.Localize((item == null ? id : item.GetDescriptionLocale())), sprite = item == null ? null : context.Sprites.Sprite(item.iconId), craftedAt = def == null ? new string[0] : def.craftsIn.Select(context.Localize).ToArray(), ingredients = new object[0] });
                    }
                    foreach (var id in tech.buildingsAfterUnlock)
                    {
                        var def = GameBalance.Me.GetDataOrNull<BuildingDef>(id);
                        var linked = def == null ? null : new LinkedEntityWidgetData(def, null, 1, false);
                        rewards.Add(new { type = "building", id, name = linked == null ? context.Localize(id) : Title(linked), description = context.Localize(id + "_d"), sprite = def == null ? null : context.Sprites.Sprite(def.BuildResultIcon), craftedAt = new string[0], ingredients = def == null ? new object[0] : Ingredients(context, def.needItems) });
                    }
                    foreach (var id in tech.townBuildingsAfterUnlock)
                    {
                        var def = GameBalance.Me.GetDataOrNull<TownBuildingDef>(id);
                        var linked = def == null ? null : new LinkedEntityWidgetData(def, null);
                        rewards.Add(new { type = "townBuilding", id, name = linked == null ? context.Localize(id) : Title(linked), description = context.Localize(id + "_d"), sprite = def == null ? null : context.Sprites.Sprite(def.BuildResultIcon), craftedAt = def == null ? new string[0] : def.craftsIn.Select(context.Localize).ToArray(), ingredients = def == null ? new object[0] : Ingredients(context, def.needItems) });
                    }
                    foreach (var id in tech.perksAfterUnlock)
                    {
                        var def = GameBalance.Me.GetDataOrNull<PerkDef>(id);
                        var linked = def == null ? null : new LinkedEntityWidgetData(def, null);
                        rewards.Add(new { type = "perk", id, name = linked == null ? context.Localize(id) : Title(linked), description = context.Localize(id + "_d"), sprite = def == null ? null : context.Sprites.Sprite(def.IconId), duration = def == null ? 0f : def.duration, craftedAt = new string[0], ingredients = new object[0] });
                    }
                    var price = tech.PriceRes.List.ToDictionary(x => x.type, x => (int)x.value);
                    var requirements = tech.districtReputationLock.List
                        .Concat(tech.CharReputationLock.List)
                        .GroupBy(x => x.type)
                        .ToDictionary(x => x.Key, x => (int)x.Sum(y => y.value));
                    var additionalEffects = new
                    {
                        addResources = tech.addGameResAfterUnlock.List.ToDictionary(x => x.type, x => x.value),
                        setResources = tech.setGameResAfterUnlock.List.ToDictionary(x => x.type, x => x.value),
                        expressions = tech.expressionsAfterUnlock.Select(x => x.ToUnparsedString()).ToArray()
                    };
                    object gate = null;
                    if (tech.techDefType == TechDefType.CharRep && tech.wgoRepLock.List.Count > 0)
                    {
                        var npc = GameBalance.Me.GetDataOrNull<WGODef>(tech.wgoRepLock.List[0].type);
                        var requirement = tech.CharReputationLock.List.FirstOrDefault();
                        gate = new { name = context.Localize(tech.wgoRepLock.List[0].type), resource = requirement.type, value = (int)requirement.value, sprite = npc == null ? null : context.Sprites.Sprite("technology-gate/" + tech.id, npc.Portrait) };
                    }
                    else if (tech.techDefType == TechDefType.DisRep && tech.districtReputationLock.List.Count > 0)
                    {
                        var requirement = tech.districtReputationLock.List[0];
                        gate = new { name = context.Localize(requirement.type), resource = requirement.type, value = (int)requirement.value, sprite = string.IsNullOrEmpty(tech.customIconId) ? null : context.Sprites.Sprite("technology-gate/" + tech.id, EasySpritesCollection.Instance.GetSprite(tech.customIconId, null)) };
                    }
                    technologies.Add(new
                    {
                        id = tech.id,
                        name = context.Localize(tech.id),
                        description = context.Localize(tech.id + "_d"),
                        tab = tech.tab.ToString(),
                        x = tech.TreePos.x,
                        y = tech.TreePos.y,
                        parents = tech.parents,
                        lockType = tech.techLockType.ToString(),
                        availableAtStart = tech.availableAtStart,
                        hiddenAtStart = tech.hiddenAtStart,
                        type = tech.techDefType.ToString(),
                        requirements,
                        additionalEffects,
                        gate,
                        icon = string.IsNullOrEmpty(tech.customIconId) ? null : context.Sprites.Sprite(tech.customIconId),
                        price,
                        rewards
                    });
                }
                catch (Exception error) { context.Warn("technology:" + tech.id, error.ToString()); }
                yield return null;
            }

            var expLevels = GameBalance.Me.talentExpLevelDefs.Select(x => new
            {
                orange = x.orange, red = x.red, green = x.green, yellow = x.yellow, blue = x.blue
            }).ToArray();
            var branchNames = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["talent_orange"] = context.Localize("tech_tab_Building"),
                ["talent_red"] = context.Localize("tech_tab_Metallurgy"),
                ["talent_green"] = context.Localize("tech_tab_Farming"),
                ["talent_yellow"] = context.Localize("tech_tab_Theology"),
                ["talent_blue"] = context.Localize("tech_tab_Anatomy")
            };
            var branches = GameBalance.Me.talentDefs.Select(t => new
            {
                id = t.id,
                name = branchNames.TryGetValue(t.id, out var branchName) ? branchName : context.Localize(t.id),
                description = context.Localize(t.id + "_d"),
                fontIcon = context.FontIcon(t.id),
                color = t.dbg_color
            }).ToArray();
            var inspirations = GameBalance.Me.inspirationDefs.OrderBy(x => x.talentId).ThenBy(x => x.idWithoutLvl).ThenBy(x => x.lvl).Select(x => new
            {
                id = x.id,
                baseId = x.idWithoutLvl,
                name = context.Localize(x.id),
                description = context.Localize(x.id + "_d"),
                talent = x.talentId,
                level = x.lvl,
                levelFrame = x.lvlFrame,
                completionPrice = x.completionPrice,
                completionGoal = x.completionGoalValue,
                completionExp = x.completionExp,
                inspirationLocks = x.inpsirationLocks,
                techLocks = x.techLocks,
                questLocks = x.questLocks,
                sprite = context.Sprites.Sprite("inspiration/" + x.id, x.Icon)
            }).ToArray();
            var levelUps = GameBalance.Me.talentLevelUpDefs.Where(x => !x.isZombiePerk).OrderBy(x => x.talentId).ThenBy(x => x.TreePos.x).ThenBy(x => x.TreePos.y).Select(x =>
            {
                var perk = string.IsNullOrEmpty(x.linkedPerk) ? null : GameBalance.Me.GetDataOrNull<PerkDef>(x.linkedPerk);
                var nameKey = context.English.ContainsKey(x.id) ? x.id : (perk == null ? x.id : perk.id);
                var descriptionKey = context.English.ContainsKey(x.id + "_d") ? x.id + "_d" : (perk == null ? x.id + "_d" : perk.id + "_d");
                return new
                {
                    id = x.id,
                    name = context.Localize(nameKey),
                    description = context.Localize(descriptionKey),
                    talent = x.talentId,
                    x = x.TreePos.x,
                    y = x.TreePos.y,
                    parents = x.parents,
                    lockType = x.lockType.ToString(),
                    availableAtStart = x.availableAtStart,
                    hidden = x.isHidden,
                    unknown = x.isUnknown,
                    freeCoordinates = x.isFreeCoordinates,
                    talentValue = x.talentValueAdd,
                    pointPrice = x.talentExpPointsPrice,
                    sprite = context.Sprites.Sprite("talent-level/" + x.id, x.Icon),
                    perk = perk == null ? null : new { id = perk.id, name = context.Localize(perk.id), description = context.Localize(perk.id + "_d"), sprite = context.Sprites.Sprite(perk.IconId) }
                };
            }).ToArray();

            context.Write("progression.json", new
            {
                schemaVersion = 1,
                technology = new { tabs, nodes = technologies },
                talents = new { branches, expLevels, inspirations, levelUps }
            });
        }
    }
}
