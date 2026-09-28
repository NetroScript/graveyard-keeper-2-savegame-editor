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

        private static string TechnologyText(ExportContext context, string id, string suffix = "")
        {
            var key = id + suffix;
            if (context.English.ContainsKey(key)) return context.Localize(key);
            if (id.StartsWith("tech_", StringComparison.Ordinal))
            {
                var fallback = id.Substring(5) + suffix;
                if (context.English.ContainsKey(fallback)) return context.Localize(fallback);
            }
            return key;
        }

        private static string WorkstationName(ExportContext context, string id)
        {
            if (context.English.ContainsKey(id)) return context.Localize(id);
            const string prefix = "t_b_signboard_";
            if (id.StartsWith(prefix, StringComparison.Ordinal))
            {
                var categoryKey = "repair_sign_" + id.Substring(prefix.Length);
                if (context.English.ContainsKey(categoryKey)) return context.Localize(categoryKey);
            }
            return id;
        }

        private static string[] WorkstationNames(ExportContext context, IEnumerable<string> ids)
        {
            var result = new List<string>();
            var hasTownCategory = false;
            foreach (var id in ids)
            {
                if (id.StartsWith("t_b_signboard_", StringComparison.Ordinal)) hasTownCategory = true;
                else if (hasTownCategory && id.StartsWith("repair_sign_", StringComparison.Ordinal)
                    && id.Substring("repair_sign_".Length).All(char.IsDigit)) continue;
                result.Add(WorkstationName(context, id));
            }
            return result.Distinct(StringComparer.Ordinal).ToArray();
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

        private static object[] Resources(GameRes resources)
        {
            return resources == null ? new object[0] : resources.List.Select(x => (object)new { type = x.type, value = x.value }).ToArray();
        }

        private static string[] Expressions(IEnumerable<LazyExpression> expressions)
        {
            return expressions == null ? new string[0] : expressions.Select(x => x?.GetRawExpressionString()).ToArray();
        }

        private static object Perk(ExportContext context, PerkDef perk)
        {
            return new
            {
                id = perk.id,
                name = context.Localize(perk.id),
                description = context.Localize(perk.id + "_d"),
                sprite = context.Sprites.Sprite("perk/" + perk.id, perk.Icon),
                type = perk.perkType.ToString(),
                addType = perk.perkAddType.ToString(),
                perk.energyAdd,
                perk.insanityAdd,
                perk.craftStartTicks,
                perk.craftTotalProgressTicksBonus,
                perk.craftMasteryBonus,
                perk.duration,
                perk.hiddenTimer,
                perk.isHidden,
                perk.tickRate,
                perk.fertilizerItemId,
                perk.worldFxPrefabId,
                perk.hudFxPrefabId,
                effects = new
                {
                    setResourcesOnAdd = Resources(perk.setGameResOnAdd),
                    addResourcesOnAdd = Resources(perk.addGameResOnAdd),
                    expressionsOnAdd = Expressions(perk.onAddExpressions),
                    setResourcesOnRemove = Resources(perk.setGameResOnRemove),
                    addResourcesOnRemove = Resources(perk.addGameResOnRemove),
                    expressionsOnRemove = Expressions(perk.onRemoveExpressions),
                    addResourcesPerTick = Resources(perk.addGameResPerTick),
                    expressionsPerTick = Expressions(perk.onPerTickExpressions)
                }
            };
        }

        private static object LevelUp(ExportContext context, TalentLevelUpDef levelUp, IDictionary<string, object> perks)
        {
            var perk = string.IsNullOrEmpty(levelUp.linkedPerk) ? null : GameBalance.Me.GetDataOrNull<PerkDef>(levelUp.linkedPerk);
            var nameKey = context.English.ContainsKey(levelUp.id) ? levelUp.id : (perk == null ? levelUp.id : perk.id);
            var descriptionKey = context.English.ContainsKey(levelUp.id + "_d") ? levelUp.id + "_d" : (perk == null ? levelUp.id + "_d" : perk.id + "_d");
            object perkData = null;
            if (perk != null) perks.TryGetValue(perk.id, out perkData);
            return new
            {
                id = levelUp.id,
                name = context.Localize(nameKey),
                description = context.Localize(descriptionKey),
                talent = levelUp.talentId,
                x = levelUp.TreePos.x,
                y = levelUp.TreePos.y,
                parents = levelUp.parents,
                lockType = levelUp.lockType.ToString(),
                availableAtStart = levelUp.availableAtStart,
                hidden = levelUp.isHidden,
                unknown = levelUp.isUnknown,
                freeCoordinates = levelUp.isFreeCoordinates,
                talentValue = levelUp.talentValueAdd,
                pointPrice = levelUp.talentExpPointsPrice,
                technologyPrice = new { red = levelUp.techRed, green = levelUp.techGreen, blue = levelUp.techBlue },
                expressionsOnBuy = Expressions(levelUp.expressionsOnBuy),
                sprite = context.Sprites.Sprite("talent-level/" + levelUp.id, levelUp.Icon),
                perkId = levelUp.linkedPerk,
                perk = perkData
            };
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
                        rewards.Add(new { type = "craft", id, name = Title(linked), description = item == null ? context.Localize(id + "_d") : context.Localize(item.GetDescriptionLocale()), sprite = context.Sprites.Sprite(def.GetCraftResultIcon(null)), craftedAt = WorkstationNames(context, def.craftsIn), ingredients = Ingredients(context, def.needItems) });
                    }
                    foreach (var id in tech.alchemyFormulasAfterUnlock)
                    {
                        var def = GameBalance.Me.GetDataOrNull<AlchemyFormulaDef>(id);
                        var item = def == null ? null : def.ItemDef;
                        var linked = def == null ? null : new LinkedEntityWidgetData(def, null);
                        rewards.Add(new { type = "alchemy", id, name = linked == null ? context.Localize(id) : Title(linked), description = context.Localize((item == null ? id : item.GetDescriptionLocale())), sprite = item == null ? null : context.Sprites.Sprite(item.iconId), craftedAt = def == null ? new string[0] : WorkstationNames(context, def.craftsIn), ingredients = new object[0] });
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
                        rewards.Add(new { type = "townBuilding", id, name = linked == null ? context.Localize(id) : Title(linked), description = context.Localize(id + "_d"), sprite = def == null ? null : context.Sprites.Sprite(def.BuildResultIcon), craftedAt = def == null ? new string[0] : WorkstationNames(context, def.craftsIn), ingredients = def == null ? new object[0] : Ingredients(context, def.needItems) });
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
                        name = TechnologyText(context, tech.id),
                        description = TechnologyText(context, tech.id, "_d"),
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
            var perks = GameBalance.Me.perkDefs.OrderBy(x => x.id, StringComparer.Ordinal)
                .ToDictionary(x => x.id, x => Perk(context, x), StringComparer.Ordinal);
            var orderedLevelUps = GameBalance.Me.talentLevelUpDefs.OrderBy(x => x.talentId)
                .ThenBy(x => x.TreePos.x).ThenBy(x => x.TreePos.y).ToArray();
            var levelUps = orderedLevelUps.Where(x => !x.isZombiePerk).Select(x => LevelUp(context, x, perks)).ToArray();
            var zombieLevelUps = orderedLevelUps.Where(x => x.isZombiePerk).Select(x => LevelUp(context, x, perks)).ToArray();

            context.Write("progression.json", new
            {
                schemaVersion = 1,
                technology = new { tabs, nodes = technologies },
                perks,
                talents = new { branches, expLevels, inspirations, levelUps, zombieLevelUps }
            });
        }
    }
}
