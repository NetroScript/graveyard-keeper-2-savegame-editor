using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using LazyBearTechnology;
using UnityEngine;

namespace Gk2.AssetExporter
{
    /// <summary>Exports immutable balance data needed to inspect and edit zombie workers.</summary>
    public sealed class ZombiesModule : IExportModule
    {
        public string Id => "zombies";
        public int Order => 27;

        private const BindingFlags InstanceFields = BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;

        private static string Expression(LazyExpression expression) => expression?.GetRawExpressionString();
        private static string[] Expressions(IEnumerable<LazyExpression> expressions) =>
            expressions == null ? new string[0] : expressions.Select(Expression).ToArray();

        private static object[] Resources(GameRes resources) => resources == null
            ? new object[0]
            : resources.List.Select(x => (object)new { type = x.type, value = x.value }).ToArray();

        private static object Need(NeedItemData item) => new
        {
            id = item.id,
            groupType = item.groupType.ToString(),
            countExpression = Expression(item.count)
        };

        private static object Chance(ChanceOutputItem item) => new
        {
            id = item.id,
            item.outputGroupId,
            countExpression = Expression(item.count),
            minExpression = Expression(item.minValue),
            maxExpression = Expression(item.maxValue),
            chanceExpression = Expression(item.chance),
            item.isStarGroup
        };

        private static object Output(OutputItems output) => output == null ? null : new
        {
            items = output.chanceOutputItems.Select(Chance).ToArray(),
            groups = output.groupChanceOutputItems.Select(group => new
            {
                group.outputGroupId,
                items = group.chanceItems.Select(Chance).ToArray()
            }).ToArray()
        };

        private static object Body(ExportContext context, BodyDef body) => new
        {
            id = body.id,
            name = context.Localize(body.id),
            body.linkedBodyItemId,
            body.tier,
            parts = body.parts.ToArray(),
            pockets = body.pocket.ToArray(),
            burialRewards = body.burialReward.ToArray(),
            body.armorId,
            body.handsId
        };

        private static object Craft(ExportContext context, CraftDef craft) => new
        {
            id = craft.id,
            name = context.Localize(craft.id),
            description = context.Localize(craft.id + "_d"),
            craftsIn = craft.craftsIn.ToArray(),
            craft.extensionNeedId,
            craft.isAuto,
            craft.isHidden,
            craft.isAutoFinish,
            craft.isConveyorCraft,
            craft.tabId,
            customAction = craft.customItemTypeAction.ToString(),
            durationExpression = Expression(craft.duration),
            energyExpression = Expression(craft.energyPerTick),
            insanityExpression = Expression(craft.insanityPerTick),
            insanityLockExpression = Expression(craft.insanityLock),
            craft.talentLock,
            linkedPerks = craft.linkedPerks.ToArray(),
            needs = craft.needItems.Select(Need).ToArray(),
            needsFromWgo = craft.needItemsFromWgo.Select(Need).ToArray(),
            output = Output(craft.outputItems),
            addItemsToWgoOnStart = Output(craft.addItemsToWgoOnStart),
            addItemsToWgoOnFinish = Output(craft.addItemsToWgoOnFinish),
            craft.isAutopsyCraft,
            craft.isPocketExtractCraft,
            autopsyType = craft.autopsyTypeCraft.ToString(),
            craft.autopsyItemId,
            removeItemsFromWgo = craft.removeItemsFromWgo.Select(Need).ToArray(),
            replaceWgoId = craft.replaceWgoId,
            craft.transferDataOnReplace,
            technologyRewards = new
            {
                red = Expression(craft.techRed),
                green = Expression(craft.techGreen),
                blue = Expression(craft.techBlue)
            },
            zombieSpeedItems = Resources(craft.zombieSpeedItemModificators),
            effects = new
            {
                setWgoOnStart = Resources(craft.setWgoParamsOnStart),
                setWgoOnFinish = Resources(craft.setWgoParamsOnFinish),
                addWgoOnStart = Resources(craft.addWgoParamsOnStart),
                addWgoOnFinish = Resources(craft.addWgoParamsOnFinish),
                onAddQueue = Expressions(craft.onCraftAddQueueExpressions),
                onStart = Expressions(craft.onCraftStartExpressions),
                onEnd = Expressions(craft.onCraftEndExpressions),
                onReplace = Expressions(craft.executeOnReplace)
            },
            craft.iconId,
            customIconExpression = Expression(craft.customCraftResultIcon)
        };

        private static object Workstation(ExportContext context, WGODef wgo) => new
        {
            id = wgo.id,
            name = context.Localize(wgo.id),
            wgo.zombieRollDataId,
            wgo.wgoGroup,
            wgo.canInsertZombie,
            wgo.isAutoCrafter,
            talent = wgo.talent,
            masteryLockExpression = Expression(wgo.masteryLock),
            actionableTool = wgo.toolAction == null ? ItemType.None.ToString() : wgo.toolAction.actionableTool.ToString(),
            wgo.inventorySize,
            wgo.craftInventorySize,
            wgo.emptyCellStackCount,
            wgo.craftIconId,
            craftIcon = string.IsNullOrEmpty(wgo.craftIconId) ? null : context.Sprites.Sprite(wgo.craftIconId),
            wgo.interactionType,
            wgo.customVisualId,
            customAssetExpression = Expression(wgo.customAssetId),
            wgo.conveyorType,
            attachedWorkbenchExtensionIds = wgo.attachedWorkbenchExtensionIds.ToArray()
        };

        private static object Fighter(FighterDef fighter) => new
        {
            id = fighter.id,
            hpExpression = Expression(fighter.hp),
            attackDamageExpression = Expression(fighter.atkDamage),
            attackRangeExpression = Expression(fighter.atkRange),
            attackPauseExpression = Expression(fighter.atkPause),
            armorExpression = Expression(fighter.armor),
            returnDamageExpression = Expression(fighter.retDamage),
            knockbackExpression = Expression(fighter.knockbackForce),
            fighter.mvtSpeed,
            fighter.mvtAcceleration,
            fighter.killXp,
            targetFilterExpression = Expression(fighter.targetFilterDockPointTag),
            expressionsOnCombatDeath = Expressions(fighter.expressionsOnCombatDeath)
        };

        private static FieldInfo Field(Type type, string name) => type.GetField(name, InstanceFields);

        private static object TextureEntry(ExportContext context, string scope, Texture2D texture)
        {
            if (texture == null) return null;
            var key = "zombie-customization/" + scope + "/" + texture.name;
            return new { name = texture.name, texture = context.Sprites.Texture(key, texture) };
        }

        private static string PortraitSprite(ExportContext context, int id, string suffix)
        {
            var name = id + suffix;
            return context.Sprites.Sprite(name);
        }

        private static string OptionalPortraitSprite(ExportContext context, int id, string suffix)
        {
            var name = id + suffix;
            return EasySpritesCollection.Instance.HasSprite(name) ? context.Sprites.Sprite(name) : null;
        }

        private static object Customization(ExportContext context)
        {
            var config = LazySingletonSO<ZombieCustomizationConfig>.Instance;
            if (config == null) throw new InvalidOperationException("ZombieCustomizationConfig is unavailable");
            var rolledField = Field(typeof(ZombieCustomizationConfig), "zombieRolledDatas");
            var rolled = (IEnumerable)rolledField.GetValue(config);
            var sets = new List<object>();
            foreach (var entry in rolled)
            {
                var type = entry.GetType();
                var id = (string)Field(type, "id").GetValue(entry);
                var bodies = ((IEnumerable<Texture2D>)Field(type, "bodyTextures").GetValue(entry)).ToArray();
                var heads = ((IEnumerable<Texture2D>)Field(type, "headTextures").GetValue(entry)).ToArray();
                var bodyIds = ((IEnumerable<int>)Field(type, "bodyIds").GetValue(entry)).ToArray();
                var headIds = ((IEnumerable<int>)Field(type, "headIds").GetValue(entry)).ToArray();
                sets.Add(new
                {
                    id,
                    bodyIds,
                    headIds,
                    bodyVariants = bodyIds.Select(value => new
                    {
                        id = value,
                        sprite = PortraitSprite(context, value, "_bdy_static_down"),
                        overlaySprite = OptionalPortraitSprite(context, value, "_bdy_over_static_down")
                    }).ToArray(),
                    headVariants = headIds.Select(value => new
                    {
                        id = value,
                        sprite = PortraitSprite(context, value, "_hed_static_down")
                    }).ToArray(),
                    bodyLuts = bodies.Select(x => TextureEntry(context, id + "/body", x)).ToArray(),
                    headLuts = heads.Select(x => TextureEntry(context, id + "/head", x)).ToArray()
                });
            }

            var bodyPalettes = new List<object>();
            foreach (var entry in (IEnumerable)Field(typeof(ZombieCustomizationConfig), "fightersBodyPalettes").GetValue(config))
            {
                var type = entry.GetType();
                var color = Field(type, "colorType").GetValue(entry).ToString();
                var palette = (ArmorColorPalette)Field(type, "palette").GetValue(entry);
                bodyPalettes.Add(new
                {
                    color,
                    textures = palette.GetPalettes().Where(x => x != null).Select(x => TextureEntry(context, "fighter-body/" + color, x)).ToArray()
                });
            }

            var armorPalettes = new List<object>();
            foreach (var entry in (IEnumerable)Field(typeof(ZombieCustomizationConfig), "fightersArmsArmorPalettes").GetValue(config))
            {
                var type = entry.GetType();
                var tier = Field(type, "tier").GetValue(entry).ToString();
                var palette = (ArmorColorPalette)Field(type, "palette").GetValue(entry);
                armorPalettes.Add(new
                {
                    tier,
                    textures = palette.GetPalettes().Where(x => x != null).Select(x => TextureEntry(context, "fighter-armor/" + tier, x)).ToArray()
                });
            }

            return new
            {
                available = true,
                sets,
                fighterBodyPalettes = bodyPalettes,
                fighterArmorPalettes = armorPalettes,
                portrait = new
                {
                    stoneSprite = context.Sprites.Sprite("1001_stn_static_down"),
                    layerOrder = new[] { "body", "bodyOverlay", "stone", "head" },
                    lut = new { layout = "horizontal-blue-slices", size = 32, greenAxis = "up" }
                },
                saveResources = new { bodyId = "zombie_body_id", headId = "zombie_head_id", bodyLut = "zombie_body_lut", headLut = "zombie_head_lut" },
                note = "The game rolls head/body IDs and LUTs. Armor and weapons are item equipment; no separate zombie clothing catalog was found."
            };
        }

        public IEnumerator Export(ExportContext context)
        {
            var bodies = GameBalance.Me.bodyDefs.OrderBy(x => x.id, StringComparer.Ordinal).Select(x => Body(context, x)).ToArray();
            yield return null;

            var crafts = new List<object>();
            foreach (var craft in GameBalance.Me.craftDefs.OrderBy(x => x.id, StringComparer.Ordinal))
            {
                try { crafts.Add(Craft(context, craft)); }
                catch (Exception error) { context.Warn("zombie-craft:" + craft.id, error.ToString()); }
                yield return null;
            }

            var workstations = new List<object>();
            var relevantWgos = GameBalance.Me.wgoDefs.Where(x => x.canInsertZombie || x.isAutoCrafter
                || !string.IsNullOrEmpty(x.talent) || !string.IsNullOrEmpty(x.zombieRollDataId));
            foreach (var wgo in relevantWgos.OrderBy(x => x.id, StringComparer.Ordinal))
            {
                try { workstations.Add(Workstation(context, wgo)); }
                catch (Exception error) { context.Warn("zombie-workstation:" + wgo.id, error.ToString()); }
                yield return null;
            }

            object customization;
            try { customization = Customization(context); }
            catch (Exception error)
            {
                context.Warn("zombie-customization", error.ToString());
                customization = new { available = false, error = error.Message };
            }
            yield return null;

            context.Write("zombies.json", new
            {
                schemaVersion = 1,
                bodies,
                crafts,
                workstations,
                fighters = GameBalance.Me.fighterDefs.OrderBy(x => x.id, StringComparer.Ordinal).Select(Fighter).ToArray(),
                customization,
                enums = new
                {
                    zombieTypes = Enum.GetValues(typeof(ZombieType)).Cast<ZombieType>().Select(x => new { name = x.ToString(), value = (int)x }).ToArray(),
                    autopsyTypes = Enum.GetValues(typeof(AutopsyTypeCraft)).Cast<AutopsyTypeCraft>().Select(x => new { name = x.ToString(), value = (int)x }).ToArray()
                }
            });
        }
    }
}
